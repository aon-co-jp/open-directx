//! `vector_add_mul_div_chain3_dxil.hlsl`(実dxc.exe出力、DXIL/SM6.0、
//! `t1 = A[i]+B[i]; t2 = t1*A[i]; Out[i] = t2/B[i];`という2項演算**3回**の
//! チェーン)の実機検証テスト。2026-08-05増分——DXBC側の
//! `vector_add_mul_div_chain3_real_vulkan.rs`と対になるDXIL版。
//! `resolve_dxil_calls_and_chain`/`translate_dxil_chain_to_spirv`に3個目の
//! 演算専用のコードを一切追加していないことを実機で裏付ける目的のテスト。

use directx_shader_translate::translate_dxil_chain_to_spirv;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_ADD_MUL_DIV_CHAIN3_DXIL: &[u8] = include_bytes!("../shaders/vector_add_mul_div_chain3.dxil");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxil_vector_add_mul_div_chain3_matches_cpu_reference_on_real_vulkan_hardware() {
    let kernel = translate_dxil_chain_to_spirv(VECTOR_ADD_MUL_DIV_CHAIN3_DXIL)
        .expect("real dxc-compiled DXIL 3-op chain (add, mul, div) must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1), "numthreads must be extracted from METADATA_BLOCK, not hardcoded");
    assert_eq!(kernel.write_uav_bind_point, 2);
    assert_eq!(kernel.read_uav_bind_points.len(), 4, "expression tree references A,B,A,B (4 loads total, A and B each reused)");
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
    // ゼロ除算を避けるため、bは常に正の非ゼロ値にする(既存vector_divテストと
    // 同じ配慮)。
    let a: Vec<f32> = (0..N).map(|i| (i as f32) * 0.1 + 1.0).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.25 + 1.0).collect();
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
        .expect("launch_kernel (DXIL-derived SPIR-V 3-op chain, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let expected = ((a[i] + b[i]) * a[i]) / b[i];
        assert!(
            (c[i] - expected).abs() < 1e-1,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXIL(dxc.exe実コンパイル、SM6.0、2項演算3回のチェーン add+mul+div)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで式木を実解決)->実Vulkan経路が、CPU参照実装(((a[i]+b[i])*a[i])/b[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
