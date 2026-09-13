//! FFv1レンジコーダーの32レーン並列化、実機検証。
//!
//! `range_coder::build_range_decoder_parallel_kernel`は、FFmpeg本家の
//! 実ソース(`libavcodec/vulkan/rangecoder.glsl`、2026-09-13に実際に
//! fetchして確認)が使う**共有メモリ(`shared`/`Workgroup`)+バリア**
//! 方式に倣う——32本のinvocationがそれぞれ自分の担当コンテキストの
//! 状態を並列に共有メモリへ書き込み、バリアの後、invocation 0だけが
//! 実際の`get_rac`逐次更新を行い、再度バリアの後、32本が並列に結果を
//! 書き戻す。
//!
//! **正直な訂正**: 前回のセッションでは「32レーンsubgroup shuffle」が
//! FFv1レンジコーダーの並列化機構だと想定して`subgroup_shuffle_real_
//! vulkan.rs`を実装したが、FFmpeg本家の実ソースを実際に読んだところ
//! 誤りだったと判明した——実際の機構はsubgroup shuffleではなく
//! ワークグループ共有メモリ+バリアだった。subgroup shuffle自体が
//! GT730で動くことの実証(前回のテスト)は無駄ではないが、FFv1の
//! レンジコーダーが実際に使う機構ではなかった。

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

#[test]
fn parallel_range_decoder_matches_cpu_reference_across_32_contexts_on_real_vulkan_hardware() {
    let kernel = build_range_decoder_parallel_kernel(32);
    assert_eq!(kernel.local_size, (32, 1, 1));
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    let synthetic_bytes: Vec<u8> =
        vec![0x5A, 0xA3, 0x12, 0x9F, 0x00, 0xFF, 0x77, 0x88, 0x3C, 0x64, 0xB1, 0xE7, 0x0D, 0x4F, 0x9A, 0x21, 0x6C, 0xD8];
    // 32個の異なるコンテキスト、それぞれ異なる初期状態(全部128固定だと
    // 「状態がコンテキストごとに違う」ことの検証にならないため)。
    let initial_states: [u8; 32] = std::array::from_fn(|i| (64 + i * 4) as u8);

    // --- CPU参照実装: 同じ1本のRangeDecoderCpu(共有low/range/pos)を
    // コンテキスト0から31まで順に使う——GPU版のlane0が行う処理と
    // 完全に同じ順序・同じ意味。
    let one = one_state();
    let zero = zero_state();
    let mut cpu_dec = RangeDecoderCpu::new(&synthetic_bytes);
    let mut cpu_states = initial_states;
    let mut cpu_bits = [0u32; 32];
    for i in 0..32 {
        cpu_bits[i] = cpu_dec.get_rac(&mut cpu_states[i], &one, &zero);
    }

    // --- 実GPU ---
    const PADDED_LEN: usize = 256;
    let mut bytestream_u32 = vec![0u32; PADDED_LEN];
    for (i, &byte) in synthetic_bytes.iter().enumerate() {
        bytestream_u32[i] = byte as u32;
    }
    // zero_one_state[0..256)=zero_state, [256..512)=one_state
    // (FFmpeg実ソースと同じ1本化レイアウト)。
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

    // 1ワークグループ(32invocation)のみディスパッチする。
    let cfg = LaunchConfig::linear(kernel.local_size.0, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    // `chain_n_buffer`契約上必須の末尾`KernelArg::Usize(n)`(バッファ
    // サイズ検証用、`PADDED_LEN`に合わせる)——カーネル自体はpush
    // constantを一切使わない(全パラメータがバッファ経由かビルド時定数)。
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
        .expect("launch_kernel (parallel range decoder, real Vulkan hardware, chain_n_buffer)");
    device.synchronize().expect("synchronize");

    let mut output_u32 = vec![0u32; PADDED_LEN];
    d_output.copy_to_host(cast_u32_to_u8_mut(&mut output_u32)).expect("d2h output");
    let mut final_states_u32 = vec![0u32; PADDED_LEN];
    d_context_states.copy_to_host(cast_u32_to_u8_mut(&mut final_states_u32)).expect("d2h context_states");

    let gpu_bits: Vec<u32> = output_u32[..32].to_vec();
    assert_eq!(gpu_bits, cpu_bits.to_vec(), "GPU decoded bits (32 parallel-preloaded contexts) must match CPU reference bit-for-bit");

    let gpu_final_states: Vec<u32> = final_states_u32[..32].to_vec();
    let cpu_final_states: Vec<u32> = cpu_states.iter().map(|&s| s as u32).collect();
    assert_eq!(gpu_final_states, cpu_final_states, "GPU final per-context states must match CPU reference exactly");

    println!(
        "OK: 32レーン並列(shared memory+barrier方式、FFmpeg本家rangecoder.glslに倣う)レンジコーダーが実GT730ハードウェア上でCPU参照実装と32コンテキスト分の復号ビット・最終状態の両方で完全一致した: bits={gpu_bits:?}"
    );
}
