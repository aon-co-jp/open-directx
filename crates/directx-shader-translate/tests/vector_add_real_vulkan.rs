//! Phase 1垂直スライスの実機検証テスト。
//!
//! `fxc.exe`で実際にコンパイルされた`vector_add.hlsl`(D3D11 Compute Shader、
//! SM5.0、DXBC)を実際にパースし(`dxbc`クレート)、その`SHEX`命令列から
//! SPIR-Vを生成し(`directx_shader_translate::spirv_gen`、狭いvector_add系
//! オペコード列専用)、`open-cuda`の実`VulkanDevice`(`opencuda-vulkan`の
//! `real-vulkan`フィーチャ、`ash`経由)へディスパッチして、このマシンの実GPU
//! (NVIDIA GT 730)上で実行し、CPU参照実装の`a[i]+b[i]`と数値一致することを
//! 検証する。
//!
//! `open-cuda`の`examples/vector_add_vulkan_real`と同じ実機テストパターンに
//! 従う: 実GPUが無い/Vulkanドライバが無い環境では`eprintln!`してスキップする
//! (fakeな成功にしない)。

use directx_shader_translate::spirv_gen::translate_vector_add_shader;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_ADD_DXBC: &[u8] = include_bytes!("../shaders/vector_add.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_add_matches_cpu_reference_on_real_vulkan_hardware() {
    // 1. 実DXBCコンテナから実際にSPIR-Vを生成する(狭いvector_add系デコーダ)。
    let kernel = translate_vector_add_shader(VECTOR_ADD_DXBC)
        .expect("real fxc-compiled vector_add.dxbc must translate to SPIR-V");

    assert_eq!(
        kernel.local_size,
        (64, 1, 1),
        "vector_add.hlslのnumthreads(64,1,1)がdcl_thread_groupから正しく抽出されているはず"
    );
    assert_eq!(
        kernel.uav_bind_points,
        (0, 1, 2),
        "InputA=u0, InputB=u1, Output=u2のバインドポイントがdcl_uav_structured/ld_structured/store_structuredから正しく抽出されているはず"
    );
    assert!(
        !kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203,
        "生成したSPIR-Vの先頭ワードはリトルエンディアンマジック0x07230203のはず"
    );

    // 2. 実Vulkanデバイスを開く。実GPU/Vulkanドライバが無い環境ではスキップする
    //    (opencuda-vulkan/opencuda-directxの既存実機テストと同じ方針)。
    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    const N: usize = 256; // vector_add.hlslのnumthreads(64,1,1) x 4グループ = 256要素契約
    let a: Vec<f32> = (0..N).map(|i| i as f32).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.5).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");

    // dcl_thread_group(64,1,1)にちょうど一致するdispatch grid = N/64。
    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel
        .spirv_words
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let compiled = CompiledKernel::spirv("vector_add", kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(da.as_ptr()),
                KernelArg::Ptr(db.as_ptr()),
                KernelArg::Ptr(dc.as_ptr()),
                KernelArg::Usize(N),
            ],
        )
        .expect("launch_kernel (DXBC-derived SPIR-V, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let expected = a[i] + b[i];
        assert!(
            (c[i] - expected).abs() < 1e-3,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、\
         CPU参照実装(a[i]+b[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
