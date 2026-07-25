//! `vector_div.hlsl`(実fxc.exe出力、DXBC)の実機検証テスト。
//! `vector_mul_real_vulkan.rs`と全く同じパターン(実GPUが無ければ
//! `eprintln!`してスキップ、fakeな成功にしない)。演算がdivである点だけが
//! `vector_mul`との違い。

use directx_shader_translate::spirv_gen::translate_shader;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_DIV_DXBC: &[u8] = include_bytes!("../shaders/vector_div.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_div_matches_cpu_reference_on_real_vulkan_hardware() {
    let kernel = translate_shader(VECTOR_DIV_DXBC)
        .expect("real fxc-compiled vector_div.dxbc must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert_eq!(kernel.uav_bind_points, (0, 1, 2));
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    const N: usize = 256;
    // ゼロ除算を避けるため、両方とも常に正の非ゼロ値になるよう構成する。
    let a: Vec<f32> = (0..N).map(|i| (i as f32) * 0.1 + 1.0).collect();
    let b: Vec<f32> = (0..N).map(|i| (i as f32) * 0.05 + 2.0).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    // 注: opencuda-vulkan::VulkanDevice::launch_kernel はカーネル名で引数配線
    // を選ぶディスパッチャであり、"vector_add"以外の名前は未対応(既存の
    // vector_mul/vector_sub_boundedテストと同じ理由でこの名前を再利用する。
    // 実行される演算はSPIR-Vバイト列側で決まる)。
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
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
        let expected = a[i] / b[i];
        assert!(
            (c[i] - expected).abs() < 1e-2,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル, div)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]/b[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
