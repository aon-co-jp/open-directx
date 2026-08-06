//! DXIL(D3D12/Shader Model 6.0)版vector_addの実機検証テスト。
//!
//! `dxc.exe -T cs_6_0`で実際にコンパイルされた`vector_add.dxil`
//! (`shaders/vector_add_dxil.hlsl`、DXBC版と意図的に同一契約)を実際に
//! パースし(`dxbc`クレートのDXILチャンク解析 + `llvm-bitcode`クレートの
//! bitstream解析)、型テーブル解決 -> 命令列デコード -> 7個の`Call`命令の
//! 意味解決(`dxil::resolve_vector_add_dxil_calls`)を経て、
//! `directx_shader_translate::translate_dxil_vector_add_to_spirv`で
//! SPIR-Vを生成し、`open-cuda`の実`VulkanDevice`
//! (`opencuda-vulkan`の`real-vulkan`フィーチャ、`ash`経由)へディスパッチして、
//! このマシンの実GPU(NVIDIA GT 730)上で実行し、CPU参照実装の`a[i]+b[i]`と
//! 数値一致することを検証する。DXBC版(`vector_add_real_vulkan.rs`)と
//! 同じ実機テストパターンに従う: 実GPUが無い/Vulkanドライバが無い環境では
//! `eprintln!`してスキップする(fakeな成功にしない)。

use directx_shader_translate::translate_dxil_vector_add_to_spirv;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_ADD_DXIL: &[u8] = include_bytes!("../shaders/vector_add.dxil");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxil_vector_add_matches_cpu_reference_on_real_vulkan_hardware() {
    // 1. 実DXIL(dxc.exeコンパイル)から実際にSPIR-Vを生成する
    //    (型テーブル解決 -> 命令列デコード -> Call意味解決 -> SPIR-V組み立て、
    //    このシェーダー専用の狭いパイプライン)。
    let kernel = translate_dxil_vector_add_to_spirv(VECTOR_ADD_DXIL)
        .expect("real dxc-compiled vector_add.dxil must translate to SPIR-V");

    assert_eq!(
        kernel.local_size,
        (64, 1, 1),
        "vector_add_dxil.hlslのnumthreads(64,1,1)(既知値、METADATA_BLOCK未抽出につき固定)"
    );
    // addは可換演算のため、dxc/LLVMの最適化によりBinOpのオペランド相対値
    // 参照順序が実際には(1,0)になっている(mulも同様、
    // `dxil.rs`の`resolves_mul_binop_from_real_dxc_compiled_dxil`参照)。
    // 書き込み先(u2)は常に一意なのでそのまま検証し、読み出し元2本は
    // 順序を問わず{u0,u1}の集合として検証する(数値的には可換なので
    // どちらの順でもCPU参照実装`a[i]+b[i]`と一致する)。
    let (uav_a, uav_b, uav_c) = kernel.uav_bind_points;
    assert_eq!(uav_c, 2, "Output=u2のバインドポイントがCreateHandleのrange_idから正しく解決されているはず");
    let mut read_uavs = [uav_a, uav_b];
    read_uavs.sort_unstable();
    assert_eq!(read_uavs, [0, 1], "InputA/InputBのバインドポイントは(順不同で){{u0,u1}}のはず");
    assert!(
        !kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203,
        "生成したSPIR-Vの先頭ワードはリトルエンディアンマジック0x07230203のはず"
    );

    // 2. 実Vulkanデバイスを開く。実GPU/Vulkanドライバが無い環境ではスキップする
    //    (DXBC版実機テストと同じ方針)。
    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    const N: usize = 256; // vector_add_dxil.hlslのnumthreads(64,1,1) x 4グループ = 256要素契約
    let a: Vec<f32> = (0..N).map(|i| i as f32).collect();
    let b: Vec<f32> = (0..N).map(|i| (N - i) as f32 * 0.5).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");

    da.copy_from_host(cast_f32_to_u8(&a)).expect("h2d a");
    db.copy_from_host(cast_f32_to_u8(&b)).expect("h2d b");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel
        .spirv_words
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
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
        .expect("launch_kernel (DXIL-derived SPIR-V, real Vulkan hardware)");
    device.synchronize().expect("synchronize");

    let mut c = vec![0.0f32; N];
    dc.copy_to_host(cast_f32_to_u8_mut(&mut c)).expect("d2h c");

    for i in 0..N {
        let expected = a[i] + b[i];
        assert!(
            (c[i] - expected).abs() < 1e-3,
            "mismatch at {i}: GPU produced {}, CPU reference expected {expected} (a={}, b={})",
            c[i],
            a[i],
            b[i]
        );
    }

    println!(
        "OK: DXIL(dxc.exe実コンパイル、SM6.0)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、\
         CPU参照実装(a[i]+b[i])と{N}要素すべてで数値一致した"
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
