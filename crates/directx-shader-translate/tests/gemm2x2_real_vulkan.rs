//! GEMM(行列積)垂直スライスの実機検証テスト。
//!
//! `fxc.exe`で実際にコンパイルされた`gemm2x2.hlsl`(D3D11 Compute Shader、
//! SM5.0、DXBC、固定2x2×2x2=2x2の完全アンロールGEMM)を実際にパースし
//! (`dxbc`クレート)、その`SHEX`命令列が既知の固定形状と一致することを
//! 検証した上でSPIR-Vを生成し(`directx_shader_translate::spirv_gen::
//! translate_gemm2x2_shader`)、`open-cuda`の実`VulkanDevice`
//! (`opencuda-vulkan`の`real-vulkan`フィーチャ、`ash`経由)へディスパッチして、
//! このマシンの実GPU上で実行し、CPU参照実装(素朴な2x2行列積)と数値一致
//! することを検証する。
//!
//! 実GPUが無い/Vulkanドライバが無い環境では`eprintln!`してスキップする
//! (fakeな成功にしない、既存の`vector_add_real_vulkan.rs`と同じ方針)。

use directx_shader_translate::translate_gemm2x2_shader;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const GEMM2X2_DXBC: &[u8] = include_bytes!("../shaders/gemm2x2.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

/// CPU参照実装。row-major 2x2 * 2x2 = 2x2の素朴な行列積。
fn cpu_matmul_2x2(a: &[f32; 4], b: &[f32; 4]) -> [f32; 4] {
    let mut c = [0.0f32; 4];
    for i in 0..2usize {
        for j in 0..2usize {
            let mut acc = 0.0f32;
            for k in 0..2usize {
                acc += a[i * 2 + k] * b[k * 2 + j];
            }
            c[i * 2 + j] = acc;
        }
    }
    c
}

#[test]
fn dxbc_gemm2x2_matches_cpu_reference_on_real_vulkan_hardware() {
    // 1. 実DXBCコンテナから実際にSPIR-Vを生成する(gemm2x2専用の固定形状デコーダ)。
    let kernel = translate_gemm2x2_shader(GEMM2X2_DXBC)
        .expect("real fxc-compiled gemm2x2.dxbc must translate to SPIR-V");

    assert_eq!(
        kernel.local_size,
        (2, 2, 1),
        "gemm2x2.hlslのnumthreads(2,2,1)がdcl_thread_groupから正しく抽出されているはず"
    );
    assert_eq!(
        kernel.uav_bind_points,
        (0, 1, 2),
        "A=u0, B=u1, C=u2のバインドポイントがdcl_uav_structuredから正しく抽出されているはず"
    );
    assert!(
        !kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203,
        "生成したSPIR-Vの先頭ワードはリトルエンディアンマジック0x07230203のはず"
    );

    // 2. 実Vulkanデバイスを開く。実GPU/Vulkanドライバが無い環境ではスキップする。
    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    // A・Bは非対称な値にして、行/列インデックスの取り違えバグ(転置ミス等)を
    // 確実に検出できるようにする(対称行列や単位行列では検出できない)。
    let a: [f32; 4] = [1.0, 2.0, 3.0, 4.0]; // [[1,2],[3,4]]
    let b: [f32; 4] = [5.0, 6.0, 7.0, 8.0]; // [[5,6],[7,8]]
    let expected = cpu_matmul_2x2(&a, &b); // [[19,22],[43,50]]

    let bytes = 4 * std::mem::size_of::<f32>();
    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");

    // numthreads(2,2,1)がそのまま1ワークグループで出力2x2全体を覆う。
    let cfg = LaunchConfig::grid2d(2, 2, 2, 2);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    // opencuda-vulkanの"vector_add"カーネル契約(3バッファ+push constant u32
    // 1個)をそのまま流用する(gemm2x2は固定サイズのためm/k/nパラメータは
    // 不要、pushconstantはシェーダー側で未使用のダミー)。
    let compiled = CompiledKernel::spirv("vector_add", kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(da.as_ptr()),
                KernelArg::Ptr(db.as_ptr()),
                KernelArg::Ptr(dc.as_ptr()),
                KernelArg::Usize(4),
            ],
        )
        .expect("launch_kernel (DXBC-derived GEMM SPIR-V, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = [0.0f32; 4];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..4 {
        assert!(
            (c[i] - expected[i]).abs() < 1e-3,
            "mismatch at {i}: GPU produced {}, CPU reference expected {} (full GPU={:?}, CPU={:?})",
            c[i],
            expected[i],
            c,
            expected
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル gemm2x2.hlsl)->SPIR-V(自前生成)->実Vulkan経路が、\
         CPU参照実装(2x2行列積)と数値一致した: GPU={c:?}, CPU={expected:?}"
    );
}
