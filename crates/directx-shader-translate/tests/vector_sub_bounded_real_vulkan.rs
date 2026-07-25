//! `vector_sub_bounded.hlsl`(実fxc.exe出力、境界チェック付き減算)の実機
//! 検証テスト。`vector_add_real_vulkan.rs`と同じパターン(実GPUが無ければ
//! `eprintln!`してスキップ、fakeな成功にしない)に加え、**実際に境界チェック
//! (`if (id.x < N)`)が機能していること**を検証する: バッファは
//! `DISPATCHED`要素分確保・ディスパッチするが、push constantの要素数`N`は
//! それより小さい値を渡し、`N..DISPATCHED`の範囲がシェーダーによって
//! 一切書き込まれず、初期化時のセンチネル値のまま残ることを確認する。

use directx_shader_translate::spirv_gen::translate_shader;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_SUB_BOUNDED_DXBC: &[u8] = include_bytes!("../shaders/vector_sub_bounded.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_sub_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware() {
    let kernel = translate_shader(VECTOR_SUB_BOUNDED_DXBC).expect(
        "real fxc-compiled vector_sub_bounded.dxbc (negated-add sub + ult/if bounds check) must translate to SPIR-V",
    );

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

    // 5ワークグループ(64スレッド x 5 = 320)をディスパッチするが、
    // 論理要素数(push constantのN)はそれより少ない256に留める。
    // よって id.x = 256..320 のスレッドは `if (id.x < N)` を満たさず、
    // Output[256..320] には一切書き込みが起きないはず。
    const DISPATCHED: usize = 320;
    const ELEMENT_COUNT: u32 = 256;
    const SENTINEL: f32 = -1.0;

    let a: Vec<f32> = (0..DISPATCHED).map(|i| (i as f32) + 10.0).collect();
    let b: Vec<f32> = (0..DISPATCHED).map(|i| (i as f32) * 0.5).collect();
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
    // vector_mul_real_vulkan.rsと同じ理由で"vector_add"名の既存配線
    // (3バッファ + push constant 1個のuint n)を再利用する。ここでの
    // push constant `n`は、シェーダー内部の実際の境界チェック
    // (`if (id.x < ElementCount)`)にそのまま使われる値でもある
    // (この配線がSPIR-V側のpush constant `Params.n`と一致するように
    // emit_spirvを実装済み)。
    let compiled = CompiledKernel::spirv("vector_add", kernel.entry_point, spirv_bytes);

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
        .expect("launch_kernel (DXBC-derived bounded-sub SPIR-V, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; DISPATCHED];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..ELEMENT_COUNT as usize {
        let expected = a[i] - b[i];
        assert!(
            (c[i] - expected).abs() < 1e-3,
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
        "OK: DXBC(fxc.exe実コンパイル, sub+境界チェック)->SPIR-V(自前生成)->実Vulkan経路が、\
         CPU参照実装(a[i]-b[i])と有効範囲{ELEMENT_COUNT}要素すべてで数値一致し、\
         境界外の{}要素はセンチネル値のまま(書き込まれなかった)ことを確認した",
        DISPATCHED - ELEMENT_COUNT as usize
    );
    println!("c[0]={}, c[{}]={}, c[{}]={}", c[0], ELEMENT_COUNT as usize - 1, c[ELEMENT_COUNT as usize - 1], DISPATCHED - 1, c[DISPATCHED - 1]);
}
