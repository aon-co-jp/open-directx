//! `vector_add_mul_chain.hlsl`(実fxc.exe出力、DXBC、`t = A[i]+B[i]; Out[i] = t*A[i];`
//! という2項演算2回のチェーン)の実機検証テスト。既存の`vector_add_real_vulkan.rs`
//! と同じパターン(実GPU/Vulkanドライバが無ければ`eprintln!`してスキップ、
//! フェイク成功にしない)。既存4形状が使う`translate_shader`ではなく、今回
//! 新設した`translate_chain_shader`(N個の逐次2項演算パターンクラス)を使う点が
//! 異なる。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_ADD_MUL_CHAIN_DXBC: &[u8] = include_bytes!("../shaders/vector_add_mul_chain.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_add_mul_chain_matches_cpu_reference_on_real_vulkan_hardware() {
    let kernel = translate_chain_shader(VECTOR_ADD_MUL_CHAIN_DXBC)
        .expect("real fxc-compiled 2-op chain (add then mul) must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    // 式木`(A+B)*A`を辿った読み込み順(A,B,Aの順で式木に登場、決め打ちではなく
    // 実際にDXBCから構築した式木の走査結果)。
    assert_eq!(kernel.read_uav_bind_points, vec![0, 1, 0]);
    assert_eq!(kernel.write_uav_bind_point, 2);
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
    let a: Vec<f32> = (0..N).map(|i| (i as f32) * 0.1 + 1.0).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.25).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    // 注: opencuda-vulkan::VulkanDevice::launch_kernel はカーネル名で引数配線
    // (3バッファ+push constant 1個のuint)を選ぶディスパッチャで、"vector_add"
    // 以外は未対応(open-cuda側は今回変更しない方針)。本テストのSPIR-V自体は
    // 実際に`(a+b)*a`のチェーン演算を行うようDXBCから生成したものであり、
    // 引数の配線(3バッファ+push n)がvector_addと全く同じ形なので、その既存の
    // 配線経路をそのまま再利用する(実行される演算はSPIR-Vバイト列側で決まり、
    // この名前文字列では変わらない、既存のmul/div/sub_boundedテストと同じ理由)。
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
        .expect("launch_kernel (DXBC-derived SPIR-V chain, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let expected = (a[i] + b[i]) * a[i];
        assert!(
            (c[i] - expected).abs() < 1e-1,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル, 2項演算2回のチェーン)->SPIR-V(自前生成、式木の再帰翻訳)->実Vulkan経路が、CPU参照実装((a[i]+b[i])*a[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
