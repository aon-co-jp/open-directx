//! FFv1レンジコーダーの最難関部分(32レーンsubgroup shuffle)へ向けた
//! 第一歩。DXBC由来ではない、Vulkan/SPIR-V専用のプロトタイプ
//! (`spirv_gen::build_subgroup_shuffle_xor1_kernel`参照——D3D11 Compute
//! Shader(SM5.0/DXBC)にはsubgroup shuffle相当の命令が無いため、これは
//! 翻訳ではなく直接組み立てたSPIR-V)。
//!
//! `PORTING.md`に記録した通り、`vulkaninfo`はこの開発機のGT730が
//! `VK_KHR_buffer_device_address`と32レーンsubgroup shuffle
//! (`subgroupSize=32`、`SUBGROUP_FEATURE_SHUFFLE_BIT`)の両方を実際に
//! サポートしていると申告している。このテストは、その申告を鵜呑みに
//! せず、実際にこのプロジェクトのSPIR-V生成+`open-cuda`の
//! `chain_n_buffer`ディスパッチ経路を通して**本当に動くか**を検証する。

use directx_shader_translate::spirv_gen::build_subgroup_shuffle_xor1_kernel;
use directx_shader_translate::OPENCUDA_VULKAN_CHAIN_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn subgroup_shuffle_xor1_matches_expected_lane_swap_on_real_vulkan_hardware() {
    let kernel = build_subgroup_shuffle_xor1_kernel();
    assert_eq!(kernel.local_size, (32, 1, 1));
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    // subgroupSize=32のワークグループを2個(64要素)ディスパッチする——
    // 1ワークグループ内で完結する前提(local_size.x==subgroupSize)を
    // 2グループ分並べても崩れないことも合わせて確認する。
    const N: usize = 64;
    let input: Vec<f32> = (0..N).map(|i| (i as f32) * 10.0 + 1.0).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let din = alloc_buffer(&device, bytes).expect("alloc Input");
    let dout = alloc_buffer(&device, bytes).expect("alloc Output");
    din.copy_from_host(cast_f32_to_u8(&input)).expect("h2d Input");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[KernelArg::Ptr(din.as_ptr()), KernelArg::Ptr(dout.as_ptr()), KernelArg::Usize(N)],
        )
        .expect("launch_kernel (subgroup shuffle prototype, real Vulkan hardware, chain_n_buffer)");
    device.synchronize().expect("synchronize");

    let mut output = vec![0.0f32; N];
    dout.copy_to_host(cast_f32_to_u8_mut(&mut output)).expect("d2h Output");

    // 各ワークグループ(=1 subgroup、32レーン)の中で、レーン(2k)と
    // レーン(2k+1)の値が入れ替わっているはず(`lane XOR 1`)。
    for i in 0..N {
        let lane = (i % 32) as u32;
        let partner = (lane ^ 1) as usize;
        let group_base = (i / 32) * 32;
        let expected = input[group_base + partner];
        assert!(
            (output[i] - expected).abs() < 1e-4,
            "mismatch at global index {i} (lane {lane} in its workgroup): GPU produced {}, expected input[{}]={expected} (partner lane {partner})",
            output[i],
            group_base + partner
        );
    }

    println!(
        "OK: subgroup shuffle(OpGroupNonUniformShuffle、lane XOR 1)が実GT730ハードウェアで{N}要素すべて期待通りのレーン交換を行った——vulkaninfoの申告(subgroupSize=32, SUBGROUP_FEATURE_SHUFFLE_BIT)が、このプロジェクトの実際の翻訳・ディスパッチ経路でも裏付けられた"
    );
}
