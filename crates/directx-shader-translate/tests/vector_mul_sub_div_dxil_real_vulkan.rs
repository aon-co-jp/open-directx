//! DXIL(D3D12/Shader Model 6.0)版mul/sub/divの実機検証テスト。
//!
//! `vector_add_dxil_real_vulkan.rs`と同じパターン(実dxc.exeコンパイル済み
//! DXILバイト列 -> `translate_dxil_binary_op_to_spirv` -> 実`VulkanDevice`
//! ディスパッチ -> CPU参照実装との数値一致)を、2026-07-26に一般化した3演算
//! (mul/sub/div、いずれも`vector_add_dxil.hlsl`と同一形状・同一契約で演算のみ
//! 異なる`shaders/vector_{mul,sub,div}_dxil.hlsl`を`dxc.exe -T cs_6_0`で実際に
//! コンパイルしたもの)へ適用する。実GPU/Vulkanドライバが無い環境では
//! `eprintln!`してスキップする(fakeな成功にしない、既存テストと同じ方針)。

use directx_shader_translate::translate_dxil_binary_op_to_spirv;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_MUL_DXIL: &[u8] = include_bytes!("../shaders/vector_mul.dxil");
const VECTOR_SUB_DXIL: &[u8] = include_bytes!("../shaders/vector_sub.dxil");
const VECTOR_DIV_DXIL: &[u8] = include_bytes!("../shaders/vector_div.dxil");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

/// 1シェーダー分の検証を共通化する(add版と重複させない)。`expected`は
/// CPU参照実装(呼び出し側が実際の演算に応じて渡す)。
fn run_and_check(dxil_bytes: &[u8], op_name: &str, expected: impl Fn(f32, f32) -> f32) {
    let kernel = translate_dxil_binary_op_to_spirv(dxil_bytes)
        .unwrap_or_else(|e| panic!("real dxc-compiled vector_{op_name}.dxil must translate to SPIR-V: {e:#}"));

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ({op_name}): {e:#}");
            return;
        }
    };
    println!("device: {} ({op_name})", device.info().name);

    const N: usize = 256;
    // ゼロ除算を避けるため、両方とも正の非ゼロ値にする(vector_div_real_vulkan.rs
    // 〈DXBC側〉と同じ配慮)。
    let a: Vec<f32> = (0..N).map(|i| (i as f32) + 1.0).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.5 + 1.0).collect();
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
        .unwrap_or_else(|e| panic!("launch_kernel ({op_name}, DXIL-derived SPIR-V, real Vulkan hardware): {e:#}"));
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let exp = expected(a[i], b[i]);
        assert!(
            (c[i] - exp).abs() < 1e-2,
            "{op_name} mismatch at {i}: GPU produced {}, CPU reference expected {exp} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }
    println!(
        "OK: DXIL(dxc.exe実コンパイル, {op_name})->SPIR-V(自前生成、resolve_dxil_calls_and_binopで演算/オペランド順序を実解決)->実Vulkan経路が、CPU参照実装と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}

#[test]
fn dxil_vector_mul_matches_cpu_reference_on_real_vulkan_hardware() {
    run_and_check(VECTOR_MUL_DXIL, "mul", |a, b| a * b);
}

#[test]
fn dxil_vector_sub_matches_cpu_reference_on_real_vulkan_hardware() {
    // subは非可換なので、演算子順序(a - b)がresolve_dxil_calls_and_binopの
    // lhs/rhs解決を経て正しく反映されていることの実証を兼ねる。
    run_and_check(VECTOR_SUB_DXIL, "sub", |a, b| a - b);
}

#[test]
fn dxil_vector_div_matches_cpu_reference_on_real_vulkan_hardware() {
    // divも非可換(a / b != b / a)なので、subと同じくオペランド順序の
    // 正しさを実機で検証する。
    run_and_check(VECTOR_DIV_DXIL, "div", |a, b| a / b);
}
