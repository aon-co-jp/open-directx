//! CPU(AVX2 gatherバッチ版)とGPU(ワークグループ共有メモリ+バリア版)
//! を**直接**突き合わせる、実機検証。
//!
//! これまでのテストは「GPU版はCPUスカラー参照実装と一致する」
//! (`range_decoder_parallel_real_vulkan.rs`等)と「CPU SIMD版はCPU
//! 逐次版と一致する」(`range_coder.rs`内の単体テスト)を**別々に**
//! 検証していた——両方とも共通の基準(`RangeDecoderCpu`/
//! `one_state`/`zero_state`)と一致することは確認済みだったが、
//! GPUの実行結果とCPU SIMDの実行結果を同じテストの中で直接
//! 突き合わせたことは無かった。このテストはその隙間を埋める:
//! 同じ入力(バイト列+初期状態)を、実GT730ハードウェア(GPU並列版)と
//! この開発機のAVX2(CPU SIMDバッチ版)の両方に与え、**両者の出力その
//! ものを直接比較する**(どちらも参照実装と一致するから間接的に一致
//! するはず、ではなく、実際に突き合わせて確認する)。

use directx_shader_translate::range_coder::{build_range_decoder_parallel_kernel, decode_context_batch_cpu_simd, one_state, zero_state};
use directx_shader_translate::OPENCUDA_VULKAN_CHAIN_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

fn cast_u32_to_u8(v: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_u32_to_u8_mut(v: &mut [u32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn cpu_avx2_and_real_gpu_produce_identical_results_for_the_same_input() {
    const CONTEXT_SIZE: usize = 48;
    const PADDED_LEN: usize = 256;

    let bytes: Vec<u8> = (0..80u32).map(|i| ((i * 61 + 17) % 256) as u8).collect();
    let initial_states: Vec<u8> = (0..CONTEXT_SIZE).map(|i| (5 + (i * 83) % 246) as u8).collect();

    // --- CPU側(AVX2 gatherバッチ版、この開発機で実行) ---
    let mut cpu_states = initial_states.clone();
    let cpu_bits = decode_context_batch_cpu_simd(&bytes, &mut cpu_states);
    let cpu_bits_u32: Vec<u32> = cpu_bits;

    // --- GPU側(実GT730ハードウェア) ---
    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    let kernel = build_range_decoder_parallel_kernel(CONTEXT_SIZE as u32);

    let one = one_state();
    let zero = zero_state();
    let mut bytestream_u32 = vec![0u32; PADDED_LEN];
    for (i, &byte) in bytes.iter().enumerate() {
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
        .expect("launch_kernel (parallel range decoder, real Vulkan hardware, cross-check)");
    device.synchronize().expect("synchronize");

    let mut gpu_output_u32 = vec![0u32; PADDED_LEN];
    d_output.copy_to_host(cast_u32_to_u8_mut(&mut gpu_output_u32)).expect("d2h output");
    let mut gpu_final_states_u32 = vec![0u32; PADDED_LEN];
    d_context_states.copy_to_host(cast_u32_to_u8_mut(&mut gpu_final_states_u32)).expect("d2h context_states");

    let gpu_bits: Vec<u32> = gpu_output_u32[..CONTEXT_SIZE].to_vec();
    let gpu_final_states: Vec<u32> = gpu_final_states_u32[..CONTEXT_SIZE].to_vec();
    let cpu_final_states: Vec<u32> = cpu_states.iter().map(|&s| s as u32).collect();

    // --- 直接突き合わせ(参照実装を介さず、CPU実行結果とGPU実行結果を
    // そのまま比較する) ---
    assert_eq!(cpu_bits_u32, gpu_bits, "CPU(AVX2 gather)とGPU(実GT730)の復号ビットが直接一致しない");
    assert_eq!(cpu_final_states, gpu_final_states, "CPU(AVX2 gather)とGPU(実GT730)の最終状態が直接一致しない");

    println!(
        "OK: CPU(AVX2 gatherバッチ版、この開発機)とGPU(ワークグループ共有メモリ+バリア版、実GT730ハードウェア)が、\
         同じ入力に対して{CONTEXT_SIZE}コンテキスト分の復号ビット・最終状態の両方で直接一致した(参照実装を介さない直接比較)"
    );
}
