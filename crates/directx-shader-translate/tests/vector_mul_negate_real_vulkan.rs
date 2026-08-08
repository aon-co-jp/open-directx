//! `vector_mul_negate.hlsl`(実fxc.exe出力、DXBC、`Output[i] = A[i] *
//! (-B[i])`)の実機検証テスト。2026-08-08追加——複数のHANDOFFエントリで
//! 「mulのnegateフラグが立つケースは未検証」と記録されていたギャップを
//! 埋める。実SHEX命令ダンプ(`examples/dump_shex`)で`mul`命令の第1ソース
//! オペランドに`negate: true`が実際に立つことを確認した上で、
//! `spirv_gen.rs::BinaryOp::MulNeg`(`A * (-B) = -(A*B)`をOpFMul+OpFNegate
//! で正しく計算する経路)を新設して対応した。

use directx_shader_translate::spirv_gen::translate_shader;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_MUL_NEGATE_DXBC: &[u8] = include_bytes!("../shaders/vector_mul_negate.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_mul_negate_matches_cpu_reference_on_real_vulkan_hardware() {
    let kernel = translate_shader(VECTOR_MUL_NEGATE_DXBC)
        .expect("real fxc-compiled mul-with-negate-operand shader must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    const N: usize = 256;
    let a: Vec<f32> = (0..N).map(|i| (i as f32) * 0.5 + 1.0).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.25 + 1.0).collect();
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
        .expect("launch_kernel (DXBC-derived mul-negate SPIR-V, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let expected = a[i] * -b[i];
        assert!(
            (c[i] - expected).abs() < 1e-1,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル, mulのソースオペランドにnegateが立つケース A*(-B))->SPIR-V(自前生成、OpFMul+OpFNegate)->実Vulkan経路が、\
         CPU参照実装(a[i]*-b[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
