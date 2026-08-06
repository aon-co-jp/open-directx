//! `vector_add_mul_chain_bounded.hlsl`(実fxc.exe出力、DXBC、
//! `if (i < N) { t = A[i]+B[i]; Out[i] = t*A[i]; } `という「境界チェック付き
//! 2項演算チェーン」)の実機検証テスト。2026-08-06追加——既存の
//! `vector_add_mul_chain_real_vulkan.rs`(境界チェック無し)と
//! `vector_sub_bounded_real_vulkan.rs`(単一演算+境界チェック)の組み合わせに
//! 当たる、それまでどのクラスにも一致しなかった形状。`vector_sub_bounded_
//! real_vulkan.rs`と同じパターンで、実際に境界チェックが機能していること
//! (ディスパッチしたスレッド数より少ない論理要素数を渡し、その外側が
//! センチネル値のまま書き込まれないこと)も検証する。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_ADD_MUL_CHAIN_BOUNDED_DXBC: &[u8] = include_bytes!("../shaders/vector_add_mul_chain_bounded.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_add_mul_chain_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware() {
    let kernel = translate_chain_shader(VECTOR_ADD_MUL_CHAIN_BOUNDED_DXBC).expect(
        "real fxc-compiled bounded 2-op chain (cbuffer+ult+if+endif around add-then-mul) must translate to SPIR-V",
    );

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert_eq!(kernel.read_uav_bind_points, vec![0, 1, 0]);
    assert_eq!(kernel.write_uav_bind_point, 2);
    assert!(kernel.bounds_check, "このシェーダーは実際にcbuffer+ult+if+endifを持つ");
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    // vector_sub_bounded_real_vulkan.rsと同じ手法: ディスパッチ数(320)より
    // 少ない論理要素数(256)をpush constantとして渡し、id.x=256..320が
    // 境界チェックにより一切書き込まれないことを確認する。
    const DISPATCHED: usize = 320;
    const ELEMENT_COUNT: u32 = 256;
    const SENTINEL: f32 = -1.0;

    let a: Vec<f32> = (0..DISPATCHED).map(|i| (i as f32) * 0.1 + 1.0).collect();
    let b: Vec<f32> = (0..DISPATCHED).map(|i| (DISPATCHED - i) as f32 * 0.25).collect();
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
    // 既存のチェーン系実機テストと同じ理由(`opencuda-vulkan::VulkanDevice::
    // launch_kernel`はカーネル名で3バッファ+push constant 1個のuintという
    // 固定引数配線を選ぶディスパッチャで、"vector_add"以外の名前は未対応)で、
    // 既存の"vector_add"名の配線をそのまま再利用する。push constantの`n`は、
    // このシェーダーの実際の境界チェック(`if (i < ElementCount)`)にそのまま
    // 使われる値でもある。
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
        .expect("launch_kernel (DXBC-derived bounded-chain SPIR-V, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; DISPATCHED];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..ELEMENT_COUNT as usize {
        let expected = (a[i] + b[i]) * a[i];
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
        "OK: DXBC(fxc.exe実コンパイル, 境界チェック付き2項演算チェーン)->SPIR-V(自前生成、式木の再帰翻訳+OpSelectionMerge/OpBranchConditional)->実Vulkan経路が、\
         CPU参照実装((a[i]+b[i])*a[i])と有効範囲{ELEMENT_COUNT}要素すべてで数値一致し、\
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
