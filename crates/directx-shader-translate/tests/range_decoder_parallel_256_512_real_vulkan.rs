//! FFv1レンジコーダーの256レーン・512レーン並列化、実機検証
//! (ユーザーのロードマップ指示: 128の次は256、256の次は512)。
//!
//! `vulkaninfo`でこの開発機のGT730の`maxComputeWorkGroupInvocations`
//! (1536)と`maxComputeSharedMemorySize`(49152バイト)を確認した上で、
//! いずれも256/512レーンで問題無く収まることを確認して実施する
//! (512要素のu32配列でも2048バイトのみ、上限の5%未満)。

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

/// `context_size`レーンで実際に実行し、CPU参照実装と突き合わせる
/// (32/64/128レーン版と同じロジックの汎用ヘルパー)。
fn run_and_verify(context_size: usize, padded_len: usize) {
    let kernel = build_range_decoder_parallel_kernel(context_size as u32);
    assert_eq!(kernel.local_size, (context_size as u32, 1, 1));
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ({context_size}レーン): {e:#}");
            return;
        }
    };
    println!("device: {} ({context_size}レーン)", device.info().name);

    // context_size分のrefillを確実に踏むよう十分な長さのバイト列にする。
    let synthetic_bytes: Vec<u8> = (0..(padded_len as u32 - 16)).map(|i| ((i * 71 + 19) % 256) as u8).collect();
    let initial_states: Vec<u8> = (0..context_size).map(|i| (20 + (i * 97) % 216) as u8).collect();

    // --- CPU参照実装 ---
    let one = one_state();
    let zero = zero_state();
    let mut cpu_dec = RangeDecoderCpu::new(&synthetic_bytes);
    let mut cpu_states = initial_states.clone();
    let mut cpu_bits = vec![0u32; context_size];
    for i in 0..context_size {
        cpu_bits[i] = cpu_dec.get_rac(&mut cpu_states[i], &one, &zero);
    }

    // --- 実GPU ---
    let mut bytestream_u32 = vec![0u32; padded_len];
    for (i, &byte) in synthetic_bytes.iter().enumerate() {
        bytestream_u32[i] = byte as u32;
    }
    let mut zero_one_state_u32 = vec![0u32; padded_len * 2];
    for i in 0..256 {
        zero_one_state_u32[i] = zero[i] as u32;
        zero_one_state_u32[256 + i] = one[i] as u32;
    }
    let mut context_states_u32 = vec![0u32; padded_len];
    for (i, &s) in initial_states.iter().enumerate() {
        context_states_u32[i] = s as u32;
    }

    let bytes_per_buf = padded_len * std::mem::size_of::<u32>();
    let d_bytestream = alloc_buffer(&device, bytes_per_buf).expect("alloc bytestream");
    let d_zero_one_state = alloc_buffer(&device, bytes_per_buf * 2).expect("alloc zero_one_state");
    let d_context_states = alloc_buffer(&device, bytes_per_buf).expect("alloc context_states");
    let d_output = alloc_buffer(&device, bytes_per_buf).expect("alloc output");

    d_bytestream.copy_from_host(cast_u32_to_u8(&bytestream_u32)).expect("h2d bytestream");
    d_zero_one_state.copy_from_host(cast_u32_to_u8(&zero_one_state_u32)).expect("h2d zero_one_state");
    d_context_states.copy_from_host(cast_u32_to_u8(&context_states_u32)).expect("h2d context_states");

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
                KernelArg::Usize(padded_len),
            ],
        )
        .unwrap_or_else(|e| panic!("launch_kernel ({context_size}-context parallel range decoder, real Vulkan hardware): {e}"));
    device.synchronize().expect("synchronize");

    let mut output_u32 = vec![0u32; padded_len];
    d_output.copy_to_host(cast_u32_to_u8_mut(&mut output_u32)).expect("d2h output");
    let mut final_states_u32 = vec![0u32; padded_len];
    d_context_states.copy_to_host(cast_u32_to_u8_mut(&mut final_states_u32)).expect("d2h context_states");

    let gpu_bits: Vec<u32> = output_u32[..context_size].to_vec();
    assert_eq!(gpu_bits, cpu_bits, "{context_size}-context GPU decoded bits must match CPU reference bit-for-bit");

    let gpu_final_states: Vec<u32> = final_states_u32[..context_size].to_vec();
    let cpu_final_states: Vec<u32> = cpu_states.iter().map(|&s| s as u32).collect();
    assert_eq!(gpu_final_states, cpu_final_states, "{context_size}-context GPU final states must match CPU reference exactly");

    println!("OK: {context_size}レーン並列レンジコーダーが実GT730ハードウェア上でCPU参照実装と完全一致した");
}

#[test]
fn parallel_range_decoder_scales_to_256_contexts_on_real_vulkan_hardware() {
    run_and_verify(256, 512);
}

#[test]
fn parallel_range_decoder_scales_to_512_contexts_on_real_vulkan_hardware() {
    run_and_verify(512, 1024);
}

/// 2026-09-13追加(ユーザーのロードマップ指示: 512の次は1024)。
/// GT730の`maxComputeWorkGroupInvocations`(`vulkaninfo`で1536と確認済み)
/// に対し1024は収まるが余裕は少ない(残り512)——**このマシンでの
/// 現実的なワークグループサイズの上限に近づいている**ことを示す
/// マイルストーンとして、これ以上の倍増(2048)は`vulkaninfo`の
/// 上限を超えるため試みない、という判断も合わせて記録する。
#[test]
fn parallel_range_decoder_scales_to_1024_contexts_on_real_vulkan_hardware() {
    run_and_verify(1024, 2048);
}

/// 2026-09-13追加(ユーザー指示: GT730の実際のハードウェア上限
/// 〈`vulkaninfo`の`maxComputeWorkGroupInvocations`=1536〉まで実装)。
/// **これがこのGPU上で単一ワークグループとして実行できる最大の
/// レーン数**——1537以上は`vkCreateComputePipelines`等が
/// `VK_ERROR_*`を返すはずの領域であり、このテストが実際の上限ちょうど
/// を実機で踏むことで「1536で成功する・1537未満に制限は無い」ことを
/// 実証する(下回った数字を安全マージンとして選んだのではなく、
/// 実際に申告された上限そのものを試す)。
#[test]
fn parallel_range_decoder_reaches_the_real_hardware_ceiling_of_1536_contexts_on_real_vulkan_hardware() {
    run_and_verify(1536, 2048);
}
