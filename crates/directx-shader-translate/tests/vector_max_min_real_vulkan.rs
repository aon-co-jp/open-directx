//! `vector_max.hlsl`/`vector_min.hlsl`(実fxc.exe出力、DXBC、
//! `Output = max(A, B)` / `Output = min(A, B)`)の実機検証。
//!
//! MED予測器(`if (topleft>=max(left,top)) pred=min(left,top); ...`)の
//! 比較器プロトタイプ第一歩。`Max`/`Min`は比較+条件分岐への分解ではなく、
//! GLSL.std.450拡張命令セットの`FMax`/`FMin`一発として翻訳される
//! (`spirv_gen.rs`の`BinaryOp::Max`/`BinaryOp::Min`参照)。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_MAX_DXBC: &[u8] = include_bytes!("../shaders/vector_max.dxbc");
const VECTOR_MIN_DXBC: &[u8] = include_bytes!("../shaders/vector_min.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

fn run_and_check(dxbc: &[u8], expected: impl Fn(f32, f32) -> f32, label: &str) {
    let kernel =
        translate_chain_shader(dxbc).unwrap_or_else(|e| panic!("real fxc-compiled {label} chain must translate to SPIR-V: {e}"));

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ({label}): {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    const N: usize = 256;
    let a: Vec<f32> = (0..N).map(|i| ((i * 3 + 7) % 97) as f32).collect();
    let b: Vec<f32> = (0..N).map(|i| ((i * 5 + 11) % 89) as f32).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc A");
    let db_ = alloc_buffer(&device, bytes).expect("alloc B");
    let dout = alloc_buffer(&device, bytes).expect("alloc Output");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d A");
    db_.copy_from_host(cast_f32_to_u8(&b)).expect("h2d B");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(da.as_ptr()),
                KernelArg::Ptr(db_.as_ptr()),
                KernelArg::Ptr(dout.as_ptr()),
                KernelArg::Usize(N),
            ],
        )
        .unwrap_or_else(|e| panic!("launch_kernel ({label}, real Vulkan hardware): {e}"));
    device.synchronize().expect("synchronize");

    let mut out = vec![0.0f32; N];
    dout.copy_to_host(cast_f32_to_u8_mut(&mut out)).expect("d2h Output");

    for i in 0..N {
        let want = expected(a[i], b[i]);
        assert!(
            (out[i] - want).abs() < 1e-4,
            "{label} mismatch at {i}: GPU produced {}, expected {want} (a={}, b={})",
            out[i],
            a[i],
            b[i]
        );
    }

    println!("OK: DXBC(fxc.exe実コンパイル、{label})->SPIR-V(GLSL.std.450 FMax/FMin)->実Vulkan経路が、CPU参照実装と{N}要素すべてで数値一致した");
}

#[test]
fn dxbc_vector_max_matches_reference_on_real_vulkan_hardware() {
    run_and_check(VECTOR_MAX_DXBC, f32::max, "vector_max (Output=max(A,B))");
}

#[test]
fn dxbc_vector_min_matches_reference_on_real_vulkan_hardware() {
    run_and_check(VECTOR_MIN_DXBC, f32::min, "vector_min (Output=min(A,B))");
}
