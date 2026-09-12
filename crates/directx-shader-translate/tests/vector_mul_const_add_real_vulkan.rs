//! `vector_mul_const_add.hlsl`(実fxc.exe出力、DXBC、
//! `Output[i] = InputA[i] * 1.402 + InputB[i]`)の実機検証テスト。
//!
//! **これが検証する内容(2026-09-12追加)**: `RegExpr`に即値定数
//! (`Immediate(f32)`)対応を追加したことで、`decode_chain_shape`が
//! `l(1.402)`のような`RegisterType::Immediate32`オペランドを含む
//! チェーン式を正しく認識し、`emit_chain_spirv`が`OpConstant`を
//! 正しく発行して実GPU上で数値的に正しい結果になることを検証する。
//! make-diskのyuv_to_rgb変換係数(1.402, 0.344136等)のような定数を
//! 含む変換カーネルへ道を開くための、最小構成での事前検証。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_MUL_CONST_ADD_DXBC: &[u8] = include_bytes!("../shaders/vector_mul_const_add.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_mul_const_add_matches_cpu_reference_on_real_vulkan_hardware() {
    let kernel = translate_chain_shader(VECTOR_MUL_CONST_ADD_DXBC)
        .expect("real fxc-compiled chain with an immediate constant operand must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    // 式木`(A*1.402)+B`の読み込み順(即値は`collect_loads`に現れないため、
    // UAVの読み込みはA, Bの順のみ)。
    assert_eq!(kernel.read_uav_bind_points, vec![0, 1]);
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
    let a: Vec<f32> = (0..N).map(|i| i as f32 * 0.5).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.25).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
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
                KernelArg::Usize(N),
            ],
        )
        .expect("launch_kernel (DXBC-derived SPIR-V chain with immediate constant, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let expected = a[i] * 1.402 + b[i];
        assert!(
            (c[i] - expected).abs() < 1e-3,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル, 即値定数1.402を含むチェーン A*1.402+B)->SPIR-V(OpConstant発行込みで自前生成)->実Vulkan経路が、CPU参照実装(a[i]*1.402+b[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
