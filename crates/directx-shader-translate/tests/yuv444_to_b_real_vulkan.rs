//! `yuv444_to_b.hlsl`(実fxc.exe出力、DXBC、`B = Y + 1.772*(U-128)`)の実機検証。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const YUV444_TO_B_DXBC: &[u8] = include_bytes!("../shaders/yuv444_to_b.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_yuv444_to_b_matches_bt601_reference_on_real_vulkan_hardware() {
    let kernel = translate_chain_shader(YUV444_TO_B_DXBC)
        .expect("real fxc-compiled Y+1.772*(U-128) chain must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
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
    let y: Vec<f32> = (0..N).map(|i| (i % 256) as f32).collect();
    let u: Vec<f32> = (0..N).map(|i| ((i * 2 + 10) % 256) as f32).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let dy = alloc_buffer(&device, bytes).expect("alloc Y");
    let du = alloc_buffer(&device, bytes).expect("alloc U");
    let db = alloc_buffer(&device, bytes).expect("alloc B (output)");

    dy.copy_from_host(cast_f32_to_u8(&y)).expect("h2d Y");
    du.copy_from_host(cast_f32_to_u8(&u)).expect("h2d U");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(dy.as_ptr()),
                KernelArg::Ptr(du.as_ptr()),
                KernelArg::Ptr(db.as_ptr()),
                KernelArg::Usize(N),
            ],
        )
        .expect("launch_kernel (DXBC-derived SPIR-V, yuv444_to_b, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut b = vec![0.0f32; N];
    db.copy_to_host(cast_f32_to_u8_mut(&mut b)).expect("d2h B");

    for i in 0..N {
        let expected = y[i] + 1.772 * (u[i] - 128.0);
        assert!(
            (b[i] - expected).abs() < 1e-2,
            "mismatch at {i}: GPU produced {}, BT.601 reference expected {expected} (y={}, u={})",
            b[i],
            y[i],
            u[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル、Y+1.772*(U-128)のBT.601 Bチャンネル計算)->SPIR-V->実Vulkan経路が、CPU参照実装と{N}要素すべてで数値一致した"
    );
    println!("b[0]={}, b[{}]={}", b[0], N - 1, b[N - 1]);
}
