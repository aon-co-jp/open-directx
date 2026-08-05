//! `vector_sub_div_chain_dxil.hlsl`(実dxc.exe出力、DXIL/SM6.0、
//! `t = A[i]-B[i]; Out[i] = t/A[i];`という2項演算2回のチェーン、sub/div版)の
//! 実機検証テスト。DXBC側`vector_sub_div_chain_real_vulkan.rs`のDXIL版。

use directx_shader_translate::translate_dxil_chain_to_spirv;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_SUB_DIV_CHAIN_DXIL: &[u8] = include_bytes!("../shaders/vector_sub_div_chain.dxil");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxil_vector_sub_div_chain_matches_cpu_reference_on_real_vulkan_hardware() {
    let kernel = translate_dxil_chain_to_spirv(VECTOR_SUB_DIV_CHAIN_DXIL)
        .expect("real dxc-compiled DXIL 2-op chain (sub then div) must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert_eq!(kernel.write_uav_bind_point, 2);
    assert_eq!(kernel.read_uav_bind_points.len(), 3, "expression tree references A,B,A (3 loads total, A reused)");
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
    // AがゼロやBに近い値だと(A-B)/Aが不安定/ゼロ除算になりうるため、
    // DXBC側の同種テストと同じくAを十分大きく保つ。
    let a: Vec<f32> = (0..N).map(|i| (i as f32) * 0.1 + 10.0).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.05).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
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
        .expect("launch_kernel (DXIL-derived SPIR-V chain, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let expected = (a[i] - b[i]) / a[i];
        assert!(
            (c[i] - expected).abs() < 1e-3,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXIL(dxc.exe実コンパイル、SM6.0、2項演算2回のチェーン sub+div)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで式木を実解決)->実Vulkan経路が、CPU参照実装((a[i]-b[i])/a[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
