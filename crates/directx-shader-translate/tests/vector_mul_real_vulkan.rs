//! `vector_mul.hlsl`(実fxc.exe出力、DXBC)の実機検証テスト。
//! `vector_add_real_vulkan.rs`と全く同じパターン(実GPUが無ければ
//! `eprintln!`してスキップ、fakeな成功にしない)。演算がmulである点だけが
//! `vector_add`との違い。

use directx_shader_translate::spirv_gen::translate_shader;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_MUL_DXBC: &[u8] = include_bytes!("../shaders/vector_mul.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_vector_mul_matches_cpu_reference_on_real_vulkan_hardware() {
    let kernel = translate_shader(VECTOR_MUL_DXBC)
        .expect("real fxc-compiled vector_mul.dxbc must translate to SPIR-V");

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
    // 注: opencuda-vulkan::VulkanDevice::launch_kernel は現状カーネル名で
    // 引数配線(3バッファ+push constant 1個のuint)を選ぶディスパッチャで
    // あり、"vector_add"以外の名前は未対応(run_vector_add_spirv/
    // run_matmul_spirvのみ実装、opencuda-vulkanのソース参照)。本テストの
    // SPIR-V自体は実際にmul演算を行うようDXBCから生成したものであり、
    // 引数の配線(3バッファ+push n)がvector_addと全く同じ形なので、その
    // 既存の配線経路をそのまま再利用する(実行される演算はSPIR-Vバイト列
    // 側で決まり、この名前文字列では変わらない)。
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
        .expect("launch_kernel (DXBC-derived SPIR-V, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let expected = a[i] * b[i];
        assert!(
            (c[i] - expected).abs() < 1e-2,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル, mul)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]*b[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
