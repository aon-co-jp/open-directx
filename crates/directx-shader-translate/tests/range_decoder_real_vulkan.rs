//! FFv1レンジコーダーの状態遷移テーブル+`get_rac`本体、実機検証。
//!
//! `range_coder::build_range_decoder_kernel`(DXBC由来ではない、
//! `rspirv`で直接組み立てたSPIR-V——`OpLoopMerge`によるループを含む
//! ため、`subgroup_shuffle_real_vulkan.rs`と同じ理由でDXBC翻訳の
//! スコープ外)が、CPU参照実装(`range_coder::RangeDecoderCpu`)と
//! ビット単位で完全に一致する復号結果を実GT730ハードウェア上で
//! 生成することを確認する。
//!
//! **正直な開示**: これは1invocationによる逐次実行であり、FFmpeg本家の
//! 32レーン並列化そのものはまだ実装していない——状態遷移テーブル駆動の
//! 適応ロジック本体が、CPU参照実装と数値的に完全一致する形でGPU上でも
//! 正しく動くことを検証する土台。

use directx_shader_translate::range_coder::{build_range_decoder_kernel, one_state, zero_state, RangeDecoderCpu};
use directx_shader_translate::OPENCUDA_VULKAN_CHAIN_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

fn cast_u32_to_u8(v: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_u32_to_u8_mut(v: &mut [u32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

const NUM_SYMBOLS: usize = 32;
const INITIAL_STATE: u32 = 128;

#[test]
fn range_decoder_matches_cpu_reference_bit_for_bit_on_real_vulkan_hardware() {
    let kernel = build_range_decoder_kernel(INITIAL_STATE, NUM_SYMBOLS as u32);
    assert_eq!(kernel.local_size, (1, 1, 1));
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    // 合成バイト列(実際のFFv1圧縮データではない——状態遷移テーブル
    // 駆動の適応ロジック自体がCPU/GPUで一致することの検証が目的)。
    let synthetic_bytes: Vec<u8> =
        vec![0x5A, 0xA3, 0x12, 0x9F, 0x00, 0xFF, 0x77, 0x88, 0x3C, 0x64, 0xB1, 0xE7, 0x0D, 0x4F, 0x9A, 0x21, 0x6C, 0xD8];

    // --- CPU参照実装 ---
    let one = one_state();
    let zero = zero_state();
    let mut cpu_dec = RangeDecoderCpu::new(&synthetic_bytes);
    let mut cpu_state = INITIAL_STATE as u8;
    let mut cpu_bits = Vec::with_capacity(NUM_SYMBOLS);
    for _ in 0..NUM_SYMBOLS {
        cpu_bits.push(cpu_dec.get_rac(&mut cpu_state, &one, &zero));
    }

    // --- 実GPU ---
    // ensure_chain_n_buffer_args(open-cuda)は「渡した最後のUsize(n)分の
    // バイト長を全バッファが満たす」ことを要求するため、256要素
    // (one_state/zero_stateの本来の長さ)に揃えてパディングする——
    // bytestream/outputは実際に使う範囲だけ意味を持ち、残りは未使用。
    const PADDED_LEN: usize = 256;
    let mut bytestream_u32 = vec![0u32; PADDED_LEN];
    for (i, &byte) in synthetic_bytes.iter().enumerate() {
        bytestream_u32[i] = byte as u32;
    }
    let one_u32: Vec<u32> = one.iter().map(|&v| v as u32).collect();
    let zero_u32: Vec<u32> = zero.iter().map(|&v| v as u32).collect();
    let bytes_per_buf = PADDED_LEN * std::mem::size_of::<u32>();

    let d_bytestream = alloc_buffer(&device, bytes_per_buf).expect("alloc bytestream");
    let d_one_state = alloc_buffer(&device, bytes_per_buf).expect("alloc one_state");
    let d_zero_state = alloc_buffer(&device, bytes_per_buf).expect("alloc zero_state");
    let d_output = alloc_buffer(&device, bytes_per_buf).expect("alloc output");

    d_bytestream.copy_from_host(cast_u32_to_u8(&bytestream_u32)).expect("h2d bytestream");
    d_one_state.copy_from_host(cast_u32_to_u8(&one_u32)).expect("h2d one_state");
    d_zero_state.copy_from_host(cast_u32_to_u8(&zero_u32)).expect("h2d zero_state");

    // 単一invocationのみでよい(状態を逐次持ち回るため)。
    let cfg = LaunchConfig::linear(1, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    // `initial_state`/`num_symbols`は`build_range_decoder_kernel`の
    // ビルド時点でSPIR-Vの`OpConstant`として直接埋め込み済み(push
    // constant経由ではない——`chain_n_buffer`が渡すpush constantは
    // 常に4バイト〈要素数n〉のみで、このカーネルが元々必要としていた
    // 8バイト〈initial_state+num_symbols〉のレイアウトとは一致しない
    // ため、レイアウト不整合を避ける設計に変更した。詳細は
    // `range_coder.rs`の`build_range_decoder_kernel`のdocコメント参照)。
    // 末尾の`KernelArg::Usize(NUM_SYMBOLS)`は`chain_n_buffer`の契約上
    // 必須の引数(バッファサイズ検証用)だが、このカーネル自体はそれを
    // 読まない(実際のループ回数は上記のビルド時定数`num_symbols`が
    // 決める)。
    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(d_bytestream.as_ptr()),
                KernelArg::Ptr(d_one_state.as_ptr()),
                KernelArg::Ptr(d_zero_state.as_ptr()),
                KernelArg::Ptr(d_output.as_ptr()),
                KernelArg::Usize(NUM_SYMBOLS),
            ],
        )
        .expect("launch_kernel (range decoder, real Vulkan hardware, chain_n_buffer)");
    device.synchronize().expect("synchronize");

    let mut output_u32 = vec![0u32; PADDED_LEN];
    d_output.copy_to_host(cast_u32_to_u8_mut(&mut output_u32)).expect("d2h output");

    let gpu_bits: Vec<u32> = output_u32[..NUM_SYMBOLS].to_vec();
    let cpu_bits_u32: Vec<u32> = cpu_bits.clone();
    assert_eq!(
        gpu_bits, cpu_bits_u32,
        "GPU decoded bits must match CPU reference bit-for-bit (initial_state={INITIAL_STATE})"
    );

    println!(
        "OK: レンジコーダーのget_rac+状態遷移テーブルが実GT730ハードウェア上でCPU参照実装と{NUM_SYMBOLS}シンボル完全一致した: {gpu_bits:?}"
    );
}
