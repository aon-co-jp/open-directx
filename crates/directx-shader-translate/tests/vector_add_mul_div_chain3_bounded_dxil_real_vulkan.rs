//! `vector_add_mul_div_chain3_bounded_dxil.hlsl`(実dxc.exe出力、DXIL/SM6.0、
//! `if (i < ElementCount) { t1 = a[i]+b[i]; t2 = t1*a[i]; c[i] = t2/b[i]; }`
//! という「境界チェック付き3項演算チェーン」)の実機検証テスト。2026-08-06
//! 追加——DXBC側`vector_add_mul_div_chain3_bounded_real_vulkan.rs`のDXIL版で、
//! `resolve_dxil_calls_and_chain`の境界チェック検出ロジック(2026-08-06に
//! 2項チェーン向けに追加済み)が3項チェーンでも無改修で通用するかを実際に
//! 検証する。

use directx_shader_translate::dxil::translate_dxil_chain_to_spirv;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_ADD_MUL_DIV_CHAIN3_BOUNDED_DXIL: &[u8] =
    include_bytes!("../shaders/vector_add_mul_div_chain3_bounded.dxil");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxil_vector_add_mul_div_chain3_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware() {
    let kernel = translate_dxil_chain_to_spirv(VECTOR_ADD_MUL_DIV_CHAIN3_BOUNDED_DXIL).expect(
        "real dxc-compiled bounded 3-op chain DXIL (cbuffer+icmp ult+conditional br around add-then-mul-then-div) must translate to SPIR-V",
    );

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert_eq!(kernel.write_uav_bind_point, 2);
    assert!(kernel.bounds_check, "このDXILシェーダーは実際にcbufferLoadLegacy+icmp ult+条件分岐+無条件分岐を持つ");
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    const DISPATCHED: usize = 320;
    const ELEMENT_COUNT: u32 = 256;
    const SENTINEL: f32 = -1.0;

    let a: Vec<f32> = (0..DISPATCHED).map(|i| (i as f32) * 0.1 + 1.0).collect();
    let b: Vec<f32> = (0..DISPATCHED).map(|i| (DISPATCHED - i) as f32 * 0.25 + 1.0).collect();
    let bytes = DISPATCHED * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");
    let sentinel_c = vec![SENTINEL; DISPATCHED];
    dc.copy_from_host(cast_f32_to_u8(&sentinel_c)).expect("h2d sentinel c");

    let cfg = LaunchConfig::linear(DISPATCHED as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(da.as_ptr()),
                KernelArg::Ptr(db.as_ptr()),
                KernelArg::Ptr(dc.as_ptr()),
                KernelArg::Usize(ELEMENT_COUNT as usize),
            ],
        )
        .expect("launch_kernel (DXIL-derived bounded 3-op chain SPIR-V, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; DISPATCHED];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..ELEMENT_COUNT as usize {
        let t1 = a[i] + b[i];
        let t2 = t1 * a[i];
        let expected = t2 / b[i];
        assert!(
            (c[i] - expected).abs() < 1e-1,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }
    for (i, &value) in c.iter().enumerate().skip(ELEMENT_COUNT as usize) {
        assert_eq!(
            value, SENTINEL,
            "境界チェック外(i={i} >= N={ELEMENT_COUNT})のはずが書き込まれてしまった: {value}"
        );
    }

    println!(
        "OK: DXIL(dxc.exe実コンパイル、SM6.0、境界チェック付き3項演算チェーン add->mul->div)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで境界チェックも実解決)->実Vulkan経路が、\
         CPU参照実装(((a[i]+b[i])*a[i])/b[i])と有効範囲{ELEMENT_COUNT}要素すべてで数値一致し、\
         境界外の{}要素はセンチネル値のまま(書き込まれなかった)ことを確認した",
        DISPATCHED - ELEMENT_COUNT as usize
    );
    println!(
        "c[0]={}, c[{}]={}, c[{}]={}",
        c[0],
        ELEMENT_COUNT as usize - 1,
        c[ELEMENT_COUNT as usize - 1],
        DISPATCHED - 1,
        c[DISPATCHED - 1]
    );
}
