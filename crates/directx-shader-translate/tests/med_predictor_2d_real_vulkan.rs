//! MED予測器(実画像2次元インデックス版)、実機検証。
//!
//! `med2d::translate_med_predictor_2d_shader`は、`med_predictor_2d.hlsl`
//! (実`fxc.exe`出力、`x-1`/`y-1`の2次元近傍参照+境界での`center`埋め)
//! 専用のDXBC→SPIR-V翻訳器(前々回セッションで「現在のデコーダを大きく
//! 超える拡張が必要」と記録した`IMul`/`UDiv`/`IMad`/`Iadd`/`And`+実分岐
//! を、このシェーダー専用の固定形状デコーダとして今回実装した)。
//!
//! `width`/`height`はビルド時定数として焼き込む設計(`med2d.rs`の
//! `translate_med_predictor_2d_shader`のdocコメント参照)のため、
//! push constantは不要——`open-cuda`の汎用`chain_n_buffer`ディスパッチ
//! (Image/Outputの2バッファ+要素数n)でそのまま実行できる。
//!
//! CPU参照実装(Rustで書いた素直なMED、2次元インデックス込み)と、
//! 実GT730ハードウェア上のSPIR-V実行結果を、境界行・境界列・内部
//! ピクセルすべてを含む座標で突き合わせる。

use directx_shader_translate::med2d::translate_med_predictor_2d_shader;
use directx_shader_translate::OPENCUDA_VULKAN_CHAIN_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const MED_PREDICTOR_2D_DXBC: &[u8] = include_bytes!("../shaders/med_predictor_2d.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

fn med_predict_reference(left: f32, top: f32, topleft: f32) -> f32 {
    if topleft >= left.max(top) {
        left.min(top)
    } else if topleft <= left.min(top) {
        left.max(top)
    } else {
        left + top - topleft
    }
}

/// `med_predictor_2d.hlsl`のHLSLソースそのものをRustへ素直に移植した
/// CPU参照実装(境界=`center`埋めの簡略化込み)。
fn med_2d_reference(image: &[f32], width: u32) -> Vec<f32> {
    let w = width as i64;
    let mut out = vec![0.0f32; image.len()];
    for i in 0..image.len() {
        let x = (i as i64) % w;
        let y = (i as i64) / w;
        let center = image[i];
        let left = if x > 0 { image[(y * w + (x - 1)) as usize] } else { center };
        let top = if y > 0 { image[((y - 1) * w + x) as usize] } else { center };
        let topleft = if x > 0 && y > 0 { image[((y - 1) * w + (x - 1)) as usize] } else { center };
        out[i] = med_predict_reference(left, top, topleft);
    }
    out
}

#[test]
fn dxbc_med_predictor_2d_matches_reference_on_real_vulkan_hardware() {
    // 8x9の画像(境界行・境界列・内部ピクセルすべてを含む、thread_group
    // 〈64〉の倍数ぴったりではないサイズ=末尾スレッドの境界チェック
    // 〈i<width*height〉も同時に検証できる)。
    const WIDTH: u32 = 8;
    const HEIGHT: u32 = 9;
    const N: usize = (WIDTH * HEIGHT) as usize;

    let kernel = translate_med_predictor_2d_shader(MED_PREDICTOR_2D_DXBC, WIDTH, HEIGHT)
        .expect("real fxc-compiled 2D MED predictor (IMul/UDiv/IMad/Iadd/And + real If/EndIf) must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert_eq!(kernel.image_uav, 0);
    assert_eq!(kernel.output_uav, 1);
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    let image: Vec<f32> = (0..N).map(|i| ((i * 37 + 13) % 251) as f32).collect();
    let expected = med_2d_reference(&image, WIDTH);

    // `chain_n_buffer`のバッファサイズ検証(`ensure_chain_n_buffer_args`)
    // が要求する「要素数n分の4バイト」を、ディスパッチする全スレッド数
    // (thread_groupの倍数、実画像サイズNより大きい場合がある)に合わせて
    // パディングする——カーネル自体はpush constant無しでwidth*heightの
    // 境界チェックを行うため、パディング領域への書き込みは発生しない
    // (`i<width*height`の外は`if`の外、`Output`への書き込み自体が無い)。
    let dispatch_n = N.div_ceil(kernel.local_size.0 as usize) * kernel.local_size.0 as usize;
    let mut image_padded = vec![0.0f32; dispatch_n];
    image_padded[..N].copy_from_slice(&image);
    let bytes = dispatch_n * std::mem::size_of::<f32>();

    let d_image = alloc_buffer(&device, bytes).expect("alloc Image");
    let d_output = alloc_buffer(&device, bytes).expect("alloc Output");
    d_image.copy_from_host(cast_f32_to_u8(&image_padded)).expect("h2d Image");

    let cfg = LaunchConfig::linear(dispatch_n as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[KernelArg::Ptr(d_image.as_ptr()), KernelArg::Ptr(d_output.as_ptr()), KernelArg::Usize(dispatch_n)],
        )
        .expect("launch_kernel (2D MED predictor, real Vulkan hardware, chain_n_buffer)");
    device.synchronize().expect("synchronize");

    let mut output_padded = vec![0.0f32; dispatch_n];
    d_output.copy_to_host(cast_f32_to_u8_mut(&mut output_padded)).expect("d2h Output");

    for i in 0..N {
        assert!(
            (output_padded[i] - expected[i]).abs() < 1e-4,
            "mismatch at pixel {i} (x={}, y={}): GPU produced {}, expected {} (2D MED reference)",
            i as u32 % WIDTH,
            i as u32 / WIDTH,
            output_padded[i],
            expected[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル、MED予測器の実画像2次元インデックス版〈x-1/y-1、境界=center埋め〉)が実GT730ハードウェア上でCPU参照実装と{WIDTH}x{HEIGHT}={N}ピクセルすべてで数値一致した(境界行・境界列・内部ピクセルすべてを含む)"
    );
}
