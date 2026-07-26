//! `vector_sub_div_chain.hlsl`(実fxc.exe出力、DXBC、`t = A[i]-B[i]; Out[i] = t/A[i];`
//! という2項演算2回のチェーン、sub/div版)の実機検証テスト。既存の
//! `vector_add_mul_chain_real_vulkan.rs`と同じパターン(実GPU/Vulkanドライバが
//! 無ければ`eprintln!`してスキップ、フェイク成功にしない)。
//!
//! **これが検証する内容(2026-07-27追加)**: `decode_chain_shape`が以前は
//! add/mulのみに対応し、negateフラグ(sub最適化)・divは明示的に拒否
//! していたが、実際にこのシェーダーをfxc.exeでコンパイル・SHEXダンプで
//! オペランド順序を確認した上で対応を追加した。この統合テストはその
//! 変更が実GPU上で数値的に正しいことを検証する。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_SUB_DIV_CHAIN_DXBC: &[u8] = include_bytes!("../shaders/vector_sub_div_chain.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_sub_div_chain_matches_cpu_reference_on_real_vulkan_hardware() {
    let kernel = translate_chain_shader(VECTOR_SUB_DIV_CHAIN_DXBC)
        .expect("real fxc-compiled 2-op chain (sub then div) must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    // 式木`(A-B)/A`を辿った読み込み順(実際にDXBCから構築した式木の走査結果、
    // 決め打ちではない)。木の形はDiv(Sub(Load(A),Load(B)), Load(A))——
    // 左優先の先行順走査でA,B,Aの順に登場する(既存のadd_mul_chainテストの
    // `vec![0,1,0]`と同じ参照パターン、演算がsub/divに変わっただけ)。
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
    // AがゼロやBに近い値だと(A-B)/Aが不安定/ゼロ除算になりうるため、
    // Aは十分大きく保つ(CPU参照実装との比較に意味を持たせるための
    // テストデータ設計、シェーダー自体の一般性を狭めるものではない)。
    let a: Vec<f32> = (0..N).map(|i| (i as f32) * 0.1 + 10.0).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.05).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    // 注: opencuda-vulkan::VulkanDevice::launch_kernel はカーネル名で引数配線
    // (3バッファ+push constant 1個のuint)を選ぶディスパッチャで、"vector_add"
    // という名前のまま既存の配線経路を再利用する(実行される演算はSPIR-V
    // バイト列側で決まり、名前文字列では変わらない——既存のmul/div/
    // sub_bounded/add_mul_chainテストと同じ理由)。
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
        "OK: DXBC(fxc.exe実コンパイル, 2項演算2回のチェーン sub+div)->SPIR-V(自前生成、式木の再帰翻訳)->実Vulkan経路が、CPU参照実装((a[i]-b[i])/a[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
