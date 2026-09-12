//! `yuv444_to_r.hlsl`(実fxc.exe出力、DXBC、`R = Y + 1.402*(V-128)`、
//! BT.601のRチャンネル計算、4:4:4=クロマサブサンプリング無し)の実機検証。
//!
//! make-diskのyuv_to_rgb本体(open-cudaの`yuv_to_rgb_cpu`のGPU版)へ向けた
//! 段階的な最初の一歩: まず「全チャンネル同解像度(4:4:4)」という
//! 最も単純なケースで、Y+V+即値係数からRチャンネルを計算する処理が
//! 実GPU上で正しく動くことを検証する。クロマサブサンプリング
//! (U/VがY解像度の半分)対応は別途デコーダ拡張が必要な次の段階。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const YUV444_TO_R_DXBC: &[u8] = include_bytes!("../shaders/yuv444_to_r.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_yuv444_to_r_matches_bt601_reference_on_real_vulkan_hardware() {
    let kernel = translate_chain_shader(YUV444_TO_R_DXBC)
        .expect("real fxc-compiled Y+1.402*(V-128) chain must translate to SPIR-V");

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
    // Y: 0-255相当のグラデーション、V: 0-255相当の別パターン(実際の
    // 8bit映像信号の値域を模した現実的なテストデータ)。
    let y: Vec<f32> = (0..N).map(|i| (i % 256) as f32).collect();
    let v: Vec<f32> = (0..N).map(|i| ((i * 3 + 40) % 256) as f32).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let dy = alloc_buffer(&device, bytes).expect("alloc Y");
    let dv = alloc_buffer(&device, bytes).expect("alloc V");
    let dr = alloc_buffer(&device, bytes).expect("alloc R (output)");

    dy.copy_from_host(cast_f32_to_u8(&y)).expect("h2d Y");
    dv.copy_from_host(cast_f32_to_u8(&v)).expect("h2d V");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(dy.as_ptr()),
                KernelArg::Ptr(dv.as_ptr()),
                KernelArg::Ptr(dr.as_ptr()),
                KernelArg::Usize(N),
            ],
        )
        .expect("launch_kernel (DXBC-derived SPIR-V, yuv444_to_r, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut r = vec![0.0f32; N];
    dr.copy_to_host(cast_f32_to_u8_mut(&mut r)).expect("d2h R");

    for i in 0..N {
        let expected = y[i] + 1.402 * (v[i] - 128.0);
        assert!(
            (r[i] - expected).abs() < 1e-2,
            "mismatch at {i}: GPU produced {}, BT.601 reference expected {expected} (y={}, v={})",
            r[i],
            y[i],
            v[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル、Y+1.402*(V-128)のBT.601 Rチャンネル計算、4:4:4)->SPIR-V->実Vulkan経路が、CPU参照実装と{N}要素すべてで数値一致した"
    );
    println!("r[0]={}, r[{}]={}", r[0], N - 1, r[N - 1]);
}
