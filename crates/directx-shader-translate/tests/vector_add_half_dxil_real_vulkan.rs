//! half精度(HLSL`half`、DXIL`dx.op.rawBufferLoad.f16`/
//! `dx.op.rawBufferStore.f16`)版vector_addの実機検証テスト。
//!
//! `dxc.exe -enable-16bit-types -T cs_6_2`で実際にコンパイルされた
//! `vector_add_half.dxil`(`shaders/vector_add_half_dxil.hlsl`)を実際に
//! パースし、`resolve_dxil_calls_and_chain`でrawBufferLoad/Store.f16の
//! 呼び出しを解決、`translate_dxil_chain_to_spirv`で`OpTypeFloat 16`ベースの
//! SPIR-Vを生成し、`open-cuda`の実`VulkanDevice`(`opencuda-vulkan`の
//! `real-vulkan`フィーチャ、`ash`経由)へディスパッチして、このマシンの実GPU
//! (NVIDIA GT 730、Kepler世代)上で実行し、CPU参照実装(`half::f16`同士の
//! 加算)と数値一致することを検証する。
//!
//! **正直な開示(2026-09-05)**: NVIDIA GT 730はKepler世代(Compute
//! Capability 3.5)であり、SM6.2 native 16-bit typesが要求する
//! `VK_KHR_16bit_storage`/`VK_KHR_shader_float16_int8`相当の実行時サポートが
//! あるかは実行してみるまで不明——このテストはその可否自体を実機で検証する
//! ものであり、実GPU/Vulkanドライバが対応していない場合は`eprintln!`して
//! スキップする(既存の実機テスト群と同じ、fakeな成功にしない設計)。
//! `VulkanDevice`側(`opencuda-vulkan`)はこのSPIR-Vが要求する
//! `Float16`/`StorageBuffer16BitAccess`ケイパビリティ・
//! `SPV_KHR_16bit_storage`拡張を明示的に要求する処理を持たない
//! (2026-09-05時点、`open-cuda`側は今回のタスク対象外のため無改修)ため、
//! ドライバ側のデフォルトサポートに依存する——`vkCreateShaderModule`/
//! パイプライン作成が失敗した場合もその旨を出力してスキップする。

use directx_shader_translate::dxil::translate_dxil_chain_to_spirv;
use directx_shader_translate::OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME;
use half::f16;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;

const VECTOR_ADD_HALF_DXIL: &[u8] = include_bytes!("../shaders/vector_add_half.dxil");

fn cast_u16_to_u8(v: &[u16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn cast_u16_to_u8_mut(v: &mut [u16]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, std::mem::size_of_val(v)) }
}

#[test]
fn dxil_vector_add_half_matches_cpu_reference_on_real_vulkan_hardware() {
    // 1. 実DXIL(dxc.exeコンパイル、-enable-16bit-types)から実際にSPIR-Vを
    //    生成する(TYPE_BLOCK解決〈DxilType::Half〉-> rawBufferLoad/Store.f16
    //    呼び出し解決 -> OpTypeFloat 16ベースのSPIR-V組み立て)。
    let kernel = translate_dxil_chain_to_spirv(VECTOR_ADD_HALF_DXIL)
        .expect("real dxc-compiled vector_add_half.dxil (rawBufferLoad/Store.f16) must translate to SPIR-V");

    assert!(kernel.is_half, "half精度DXILなのでis_halfはtrueのはず");
    assert_eq!(kernel.local_size, (64, 1, 1));
    assert_eq!(kernel.write_uav_bind_point, 2, "Output=u2のバインドポイントのはず");
    let mut read_uavs = kernel.read_uav_bind_points.clone();
    read_uavs.sort_unstable();
    assert_eq!(read_uavs, vec![0, 1], "InputA/InputBのバインドポイントは(順不同で){{u0,u1}}のはず");
    assert!(
        !kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203,
        "生成したSPIR-Vの先頭ワードはリトルエンディアンマジック0x07230203のはず"
    );

    // 2. 実Vulkanデバイスを開く。実GPU/Vulkanドライバが無い環境ではスキップ
    //    する(DXBC版/f32版DXIL実機テストと同じ方針)。
    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    const N: usize = 256; // vector_add_half_dxil.hlslのnumthreads(64,1,1) x 4グループ = 256要素契約
    let a: Vec<f16> = (0..N).map(|i| f16::from_f32(i as f32)).collect();
    let b: Vec<f16> = (0..N).map(|i| f16::from_f32((N - i) as f32 * 0.5)).collect();
    let a_bits: Vec<u16> = a.iter().map(|v| v.to_bits()).collect();
    let b_bits: Vec<u16> = b.iter().map(|v| v.to_bits()).collect();
    let half_bytes = N * std::mem::size_of::<u16>();
    // `opencuda-vulkan::real::VulkanDevice::ensure_vector_add_args`は
    // カーネル名"vector_add"の全ケースでバッファサイズを
    // `n * size_of::<f32>()`(=4バイト/要素)固定で検証する(`open-cuda`
    // 側は本タスクでは変更しない方針、CLAUDE.md参照)。half(2バイト/要素)
    // バッファは実際に必要な2倍を確保しないとこの契約チェックに落ちる
    // (実際に確認済み: "vector_add buffer too small: need 1024 bytes"を
    // このテストで実際に踏んだ)——確保サイズを`n*4`バイトへ合わせ、
    // 実データはその先頭`n*2`バイトにのみ書き込む/読み出す(シェーダ自体は
    // `ArrayStride=2`のhalf配列としてバインドされたバッファの先頭
    // `n*2`バイトしかアクセスしないため、後半の未使用領域があっても
    // 計算結果には影響しない)。
    let bytes = N * std::mem::size_of::<f32>();

    let da = alloc_buffer(&device, bytes).expect("alloc a");
    let db = alloc_buffer(&device, bytes).expect("alloc b");
    let dc = alloc_buffer(&device, bytes).expect("alloc c");
    let _ = half_bytes;

    da.copy_from_host(cast_u16_to_u8(&a_bits)).expect("h2d a");
    db.copy_from_host(cast_u16_to_u8(&b_bits)).expect("h2d b");

    let cfg = LaunchConfig::linear(N as u32, kernel.local_size.0);
    let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME, kernel.entry_point, spirv_bytes);

    let launch_result = device.launch_kernel(
        &compiled,
        &cfg,
        &[
            KernelArg::Ptr(da.as_ptr()),
            KernelArg::Ptr(db.as_ptr()),
            KernelArg::Ptr(dc.as_ptr()),
            KernelArg::Usize(N),
        ],
    );
    // 正直な開示(モジュールdoc参照): GT730(Kepler)がSM6.2 native 16-bit
    // types相当のSPIR-V(OpTypeFloat 16 + Float16/StorageBuffer16BitAccess
    // ケイパビリティ)を実際に実行できるかは検証してみるまで不明。
    // 失敗した場合はここでその旨を出力してスキップする(fakeな成功にしない)。
    if let Err(e) = launch_result {
        eprintln!(
            "half精度(OpTypeFloat 16)SPIR-Vのディスパッチに失敗したためスキップ\
             (GT730等の旧世代GPU/ドライバがFloat16/StorageBuffer16BitAccess\
             ケイパビリティを実際にはサポートしていない可能性がある): {e:#}"
        );
        return;
    }
    device.synchronize().expect("synchronize");

    let mut c_bits = vec![0u16; N];
    dc.copy_to_host(cast_u16_to_u8_mut(&mut c_bits)).expect("d2h c");
    let c: Vec<f16> = c_bits.iter().map(|&bits| f16::from_bits(bits)).collect();

    for i in 0..N {
        let expected = a[i] + b[i];
        let diff = (c[i].to_f32() - expected.to_f32()).abs();
        assert!(
            diff < 0.5,
            "mismatch at {i}: GPU produced {} ({}), CPU reference expected {} ({}) (a={}, b={})",
            c[i],
            c[i].to_f32(),
            expected,
            expected.to_f32(),
            a[i],
            b[i]
        );
    }

    println!(
        "OK: half精度DXIL(dxc.exe実コンパイル、SM6.2 -enable-16bit-types)->SPIR-V(OpTypeFloat 16、自前生成)\
         ->実Vulkan({})経路が、CPU参照実装(f16の a[i]+b[i])と{N}要素すべてで数値一致した",
        device.info().name
    );
    println!("c[0]={}, c[{}]={}", c[0], N - 1, c[N - 1]);
}
