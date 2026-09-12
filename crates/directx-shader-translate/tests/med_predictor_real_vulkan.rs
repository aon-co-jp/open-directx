//! FFv1のMED(median edge detector)予測器、実機検証。
//!
//! `pred = (topleft>=max(left,top)) ? min(left,top)
//!        : (topleft<=min(left,top)) ? max(left,top)
//!        : left+top-topleft`
//!
//! **正直な開示(スコープ)**: 実際の画像の2次元近傍参照(`x-1`/`y-1`の
//! インデックス計算)はまだ対応していない——`yuv444_to_rgb`系と同じ
//! 簡略化で、left/top/topleftをあらかじめ切り出した3本の独立フラット
//! バッファとして受け取る。今回の目的は、MEDの核心である「3分岐の
//! 比較+選択ロジック」をDXBC→SPIR-Vへ正しく翻訳できることの検証。
//!
//! `med_predictor.hlsl`を実際に`fxc.exe`でコンパイルしたところ、
//! if/else if/elseの3分岐全体が制御フロー(分岐命令)無しで`ge`
//! (比較)+`movc`(条件付き代入)だけに平坦化されていた——`spirv_gen.rs`
//! の`RegExpr::Ge`/`RegExpr::Select`(SPIR-Vの`OpSelect`へ翻訳)が
//! これに対応する。4バッファ(Left/Top/TopLeft/Output)のため、
//! `open-cuda`に追加した汎用`chain_n_buffer`ディスパッチを使う。

use directx_shader_translate::spirv_gen::translate_chain_shader;
use directx_shader_translate::OPENCUDA_VULKAN_CHAIN_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const MED_PREDICTOR_DXBC: &[u8] = include_bytes!("../shaders/med_predictor.dxbc");

fn cast_f32_to_u8(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_f32_to_u8_mut(v: &mut [f32]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

fn med_predict_reference(left: f32, top: f32, topleft: f32) -> f32 {
    if topleft >= left.max(top) {
        left.min(top)
    } else if topleft <= left.min(top) {
        left.max(top)
    } else {
        left + top - topleft
    }
}

#[test]
fn dxbc_med_predictor_matches_reference_on_real_vulkan_hardware() {
    let kernel = translate_chain_shader(MED_PREDICTOR_DXBC)
        .expect("real fxc-compiled MED predictor (ge+movc, no control flow) must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    assert_eq!(kernel.write_uav_bind_point, 3);
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
    // MEDの3分岐すべてを実際に踏むよう、left/top/topleftの大小関係が
    // 一通りに偏らないパターンを混ぜる(単調増加だけだと1分岐しか通らない)。
    let left: Vec<f32> = (0..N).map(|i| ((i * 7 + 3) % 251) as f32).collect();
    let top: Vec<f32> = (0..N).map(|i| ((i * 11 + 17) % 233) as f32).collect();
    let topleft: Vec<f32> = (0..N).map(|i| ((i * 13 + 5) % 241) as f32).collect();
    let bytes = N * std::mem::size_of::<f32>();

    let dleft = alloc_buffer(&device, bytes).expect("alloc Left");
    let dtop = alloc_buffer(&device, bytes).expect("alloc Top");
    let dtopleft = alloc_buffer(&device, bytes).expect("alloc TopLeft");
    let dout = alloc_buffer(&device, bytes).expect("alloc Output");

    dleft.copy_from_host(cast_f32_to_u8(&left)).expect("h2d Left");
    dtop.copy_from_host(cast_f32_to_u8(&top)).expect("h2d Top");
    dtopleft.copy_from_host(cast_f32_to_u8(&topleft)).expect("h2d TopLeft");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    // binding 0=Left, 1=Top, 2=TopLeft, 3=Output(med_predictor.hlslのregister
    // 宣言順、write_uav_bind_point==3で確認済み)。
    device
        .launch_kernel(
            &compiled,
            &cfg,
            &[
                KernelArg::Ptr(dleft.as_ptr()),
                KernelArg::Ptr(dtop.as_ptr()),
                KernelArg::Ptr(dtopleft.as_ptr()),
                KernelArg::Ptr(dout.as_ptr()),
                KernelArg::Usize(N),
            ],
        )
        .expect("launch_kernel (DXBC-derived SPIR-V, med_predictor, real Vulkan hardware, chain_n_buffer)");
    device.synchronize().expect("synchronize");

    let mut out = vec![0.0f32; N];
    dout.copy_to_host(cast_f32_to_u8_mut(&mut out)).expect("d2h Output");

    let mut branch_a = 0;
    let mut branch_b = 0;
    let mut branch_c = 0;
    for i in 0..N {
        let expected = med_predict_reference(left[i], top[i], topleft[i]);
        assert!(
            (out[i] - expected).abs() < 1e-4,
            "mismatch at {i}: GPU produced {}, MED reference expected {expected} (left={}, top={}, topleft={})",
            out[i],
            left[i],
            top[i],
            topleft[i]
        );
        if topleft[i] >= left[i].max(top[i]) {
            branch_a += 1;
        } else if topleft[i] <= left[i].min(top[i]) {
            branch_b += 1;
        } else {
            branch_c += 1;
        }
    }
    // テストデータが実際に3分岐すべてを踏んでいることを確認する
    // (そうでなければ、片方の分岐にバグがあっても検出できない)。
    assert!(branch_a > 0 && branch_b > 0 && branch_c > 0, "テストデータが3分岐すべてを踏んでいない: a={branch_a}, b={branch_b}, c={branch_c}");

    println!(
        "OK: DXBC(fxc.exe実コンパイル、MED予測器、ge+movcの分岐無し平坦化、4バッファ)が実GT730ハードウェアでCPU参照実装と{N}要素すべてで数値一致した(3分岐すべてを踏んだ上で: a={branch_a}, b={branch_b}, c={branch_c})"
    );
}
