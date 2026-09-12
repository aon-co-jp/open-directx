//! `yuv444_to_g.hlsl`(実fxc.exe出力、DXBC、
//! `G = Y - 0.344136*(U-128) - 0.714136*(V-128)`)の検証。
//!
//! **これが検証する内容(2026-09-12追加)**: madの乗算オペランドに
//! `negate`フラグが立つパターン(fxc.exeが`Y - k*(x-128)`を
//! `-(x-128)*k+Y`という単一mad命令へ融合したもの)を正しく符号反転付きで
//! 式木化できることを検証する。
//!
//! **2026-09-12(同日、続き): 実GPUディスパッチに格上げした**。当初は
//! Y/U/V/Outputの4バッファを使うこのシェーダーを、`opencuda-vulkan`の
//! 公開`launch_kernel`が名前ベースディスパッチ(`"vector_add"`等)で
//! バッファ本数を種類ごとに固定していたため実GPU実行できず、構造検証
//! (SPIR-Vの形が正しいことのみ)に留めていた。その後`opencuda-vulkan`に
//! 汎用Nバッファディスパッチの公開エントリポイント
//! (`"chain_n_buffer"`/`"chain_n_buffer_f32"`、`run_chain_n_buffer_spirv`)
//! を追加したため、このテストも他の3バッファ版(yuv444_to_r/b)と同じ
//! 形で実GPU数値検証に格上げできた。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use directx_shader_translate::OPENCUDA_VULKAN_CHAIN_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const YUV444_TO_G_DXBC: &[u8] = include_bytes!("../shaders/yuv444_to_g.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxbc_yuv444_to_g_matches_bt601_reference_on_real_vulkan_hardware() {
    let kernel = translate_chain_shader(YUV444_TO_G_DXBC)
        .expect("real fxc-compiled Y-0.344136*(U-128)-0.714136*(V-128) chain must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert_eq!(kernel.write_uav_bind_point, 3);
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);
    // `read_uav_bind_points`は式木を左優先で辿った出現順(バインドポイント
    // の昇順ではない)——実際に実行して確かめたところ`[2, 1, 0]`
    // (V, U, Yの順)だった。ディスパッチ時のバッファ配列の並びは、
    // `dispatch_spirv`がbinding = 配列内インデックスで割り当てる
    // 実装のため、この出現順ではなく**実際のUAVバインドポイント番号
    // (u0=Y, u1=U, u2=V, u3=Output)の昇順**で並べる必要がある——
    // 下のディスパッチ呼び出しは`[dy, du, dv, dg]`(u0,u1,u2,u3の順)を
    // 渡しており、この`read_uav_bind_points`の出現順とは独立している。
    assert_eq!(kernel.read_uav_bind_points, vec![2, 1, 0]);

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
    let v: Vec<f32> = (0..N).map(|i| ((i * 3 + 20) % 256) as f32).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let dy = alloc_buffer(&device, bytes).expect("alloc Y");
    let du = alloc_buffer(&device, bytes).expect("alloc U");
    let dv = alloc_buffer(&device, bytes).expect("alloc V");
    let dg = alloc_buffer(&device, bytes).expect("alloc G (output)");

    dy.copy_from_host(cast_f32_to_u8(&y)).expect("h2d Y");
    du.copy_from_host(cast_f32_to_u8(&u)).expect("h2d U");
    dv.copy_from_host(cast_f32_to_u8(&v)).expect("h2d V");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    // binding 0=Y, 1=U, 2=V, 3=Output(=書き込み先)の順で並べる
    // (`kernel.write_uav_bind_point == 3`をassert済みなので固定順で良い)。
    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(dy.as_ptr()),
                KernelArg::Ptr(du.as_ptr()),
                KernelArg::Ptr(dv.as_ptr()),
                KernelArg::Ptr(dg.as_ptr()),
                KernelArg::Usize(N),
            ],
        )
        .expect("launch_kernel (DXBC-derived SPIR-V, yuv444_to_g, real Vulkan hardware, chain_n_buffer)");
    device.synchronize().expect("synchronize");

    let mut g = vec![0.0f32; N];
    dg.copy_to_host(cast_f32_to_u8_mut(&mut g)).expect("d2h G");

    for i in 0..N {
        let expected = y[i] - 0.344136 * (u[i] - 128.0) - 0.714136 * (v[i] - 128.0);
        assert!(
            (g[i] - expected).abs() < 1e-2,
            "mismatch at {i}: GPU produced {}, BT.601 reference expected {expected} (y={}, u={}, v={})",
            g[i],
            y[i],
            u[i],
            v[i]
        );
    }

    println!(
        "OK: DXBC(fxc.exe実コンパイル、mad+negateを含むGチャンネル計算、4バッファ)が実GT730ハードウェアでCPU参照実装と{N}要素すべてで数値一致した(chain_n_buffer汎用ディスパッチ経由)"
    );
}
