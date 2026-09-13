//! FFv1レンジコーダーの64レーン並列化、実機検証。
//!
//! FFmpeg本家の`CONTEXT_SIZE`は32(GT730のsubgroupSizeと同じ)固定だが、
//! `range_coder::build_range_decoder_parallel_kernel`はワークグループ
//! 共有メモリ(`Workgroup`ストレージクラス)+`OpControlBarrier`のみに
//! 依存する設計であり、subgroup幅に縛られる命令
//! (`OpGroupNonUniformShuffle`等)は使っていない。ワークグループバリアは
//! ワークグループ内の全invocation(この場合64、GT730のsubgroup幅32を
//! 超える——内部的には2 subgroup分)を対象にできるため、理論上は
//! `context_size`を64にしても正しく動くはずである、という仮説を、
//! 実際にGT730の実機で検証する(ユーザー指示によるロードマップ
//! ——「32レーンに成功したら64レーンに挑戦」)。

use directx_shader_translate::range_coder::{build_range_decoder_parallel_kernel, one_state, zero_state, RangeDecoderCpu};
use directx_shader_translate::OPENCUDA_VULKAN_CHAIN_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

fn cast_u32_to_u8(v: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_u32_to_u8_mut(v: &mut [u32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

const CONTEXT_SIZE: usize = 64;

#[test]
fn parallel_range_decoder_scales_to_64_contexts_on_real_vulkan_hardware() {
    let kernel = build_range_decoder_parallel_kernel(CONTEXT_SIZE as u32);
    assert_eq!(kernel.local_size, (64, 1, 1));
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    // 64回のrefillを確実に踏むよう、32コンテキスト版より長いバイト列にする。
    let synthetic_bytes: Vec<u8> = (0..48u32).map(|i| ((i * 37 + 11) % 256) as u8).collect();
    let initial_states: [u8; CONTEXT_SIZE] = std::array::from_fn(|i| (30 + i * 3) as u8);

    // --- CPU参照実装(32コンテキスト版と同じロジック、64個へ拡張) ---
    let one = one_state();
    let zero = zero_state();
    let mut cpu_dec = RangeDecoderCpu::new(&synthetic_bytes);
    let mut cpu_states = initial_states;
    let mut cpu_bits = [0u32; CONTEXT_SIZE];
    for i in 0..CONTEXT_SIZE {
        cpu_bits[i] = cpu_dec.get_rac(&mut cpu_states[i], &one, &zero);
    }

    // --- 実GPU ---
    const PADDED_LEN: usize = 256;
    let mut bytestream_u32 = vec![0u32; PADDED_LEN];
    for (i, &byte) in synthetic_bytes.iter().enumerate() {
        bytestream_u32[i] = byte as u32;
    }
    let mut zero_one_state_u32 = vec![0u32; PADDED_LEN * 2];
    for i in 0..256 {
        zero_one_state_u32[i] = zero[i] as u32;
        zero_one_state_u32[256 + i] = one[i] as u32;
    }
    let mut context_states_u32 = vec![0u32; PADDED_LEN];
    for (i, &s) in initial_states.iter().enumerate() {
        context_states_u32[i] = s as u32;
    }

    let bytes_per_buf = PADDED_LEN * std::mem::size_of::<u32>();
    let d_bytestream = alloc_buffer(&device, bytes_per_buf).expect("alloc bytestream");
    let d_zero_one_state = alloc_buffer(&device, bytes_per_buf * 2).expect("alloc zero_one_state");
    let d_context_states = alloc_buffer(&device, bytes_per_buf).expect("alloc context_states");
    let d_output = alloc_buffer(&device, bytes_per_buf).expect("alloc output");

    d_bytestream.copy_from_host(cast_u32_to_u8(&bytestream_u32)).expect("h2d bytestream");
    d_zero_one_state.copy_from_host(cast_u32_to_u8(&zero_one_state_u32)).expect("h2d zero_one_state");
    d_context_states.copy_from_host(cast_u32_to_u8(&context_states_u32)).expect("h2d context_states");

    // 1ワークグループ(64invocation、GT730のsubgroup幅32の2倍)のみ
    // ディスパッチする。
    let cfg = LaunchConfig::linear(kernel.local_size.0, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(d_bytestream.as_ptr()),
                KernelArg::Ptr(d_zero_one_state.as_ptr()),
                KernelArg::Ptr(d_context_states.as_ptr()),
                KernelArg::Ptr(d_output.as_ptr()),
                KernelArg::Usize(PADDED_LEN),
            ],
        )
        .expect("launch_kernel (64-context parallel range decoder, real Vulkan hardware, chain_n_buffer)");
    device.synchronize().expect("synchronize");

    let mut output_u32 = vec![0u32; PADDED_LEN];
    d_output.copy_to_host(cast_u32_to_u8_mut(&mut output_u32)).expect("d2h output");
    let mut final_states_u32 = vec![0u32; PADDED_LEN];
    d_context_states.copy_to_host(cast_u32_to_u8_mut(&mut final_states_u32)).expect("d2h context_states");

    let gpu_bits: Vec<u32> = output_u32[..CONTEXT_SIZE].to_vec();
    assert_eq!(
        gpu_bits,
        cpu_bits.to_vec(),
        "64-context GPU decoded bits must match CPU reference bit-for-bit"
    );

    let gpu_final_states: Vec<u32> = final_states_u32[..CONTEXT_SIZE].to_vec();
    let cpu_final_states: Vec<u32> = cpu_states.iter().map(|&s| s as u32).collect();
    assert_eq!(gpu_final_states, cpu_final_states, "64-context GPU final states must match CPU reference exactly");

    println!(
        "OK: 64レーン並列(GT730のsubgroup幅32の2倍、shared memory+barrier方式)レンジコーダーが実GT730ハードウェア上でCPU参照実装と64コンテキスト分の復号ビット・最終状態の両方で完全一致した"
    );
}
