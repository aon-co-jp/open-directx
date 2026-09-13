//! レンジコーダー: 1invocation逐次版 vs 32レーン並列版、実機速度比較。
//!
//! **正直な開示**: これは単発の壁時計時間比較であり、GPU側の初回
//! パイプライン構築コスト(`vkCreateComputePipelines`等、
//! `dispatch_spirv`が呼び出しごとに毎回行っている——キャッシュしない
//! 実装)を含む。統計的に厳密なベンチマーク(ウォームアップ・複数回
//! 平均・分散)ではなく、「並列版が逐次版よりオーバーヘッド込みでも
//! 明らかに遅い、ということはないか」を確認する一次スクリーニング。

use directx_shader_translate::range_coder::{build_range_decoder_kernel, build_range_decoder_parallel_kernel};
use directx_shader_translate::OPENCUDA_VULKAN_CHAIN_KERNEL_NAME;
use opencuda_core::{alloc_buffer, CompiledKernel, GpuDevice, KernelArg, LaunchConfig};
use opencuda_vulkan::VulkanDevice;
use std::time::Instant;

const PADDED_LEN: usize = 256;
const ITERATIONS: u32 = 50;

fn make_bytestream() -> Vec<u32> {
    let synthetic_bytes: Vec<u8> = (0..64u32).map(|i| ((i * 41 + 3) % 256) as u8).collect();
    let mut v = vec![0u32; PADDED_LEN];
    for (i, &byte) in synthetic_bytes.iter().enumerate() {
        v[i] = byte as u32;
    }
    v
}

#[test]
fn parallel_range_decoder_is_not_slower_than_serial_on_real_vulkan_hardware() {
    let device: std::sync::Arc<dyn GpuDevice> = match VulkanDevice::new(0) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    println!("device: {}", device.info().name);

    let bytestream_u32 = make_bytestream();
    let bytes_per_buf = PADDED_LEN * std::mem::size_of::<u32>();

    // --- 1invocation逐次版(32シンボル、build_range_decoder_kernel) ---
    let serial_kernel = build_range_decoder_kernel(128, 32);
    let d_bytestream_s = alloc_buffer(&device, bytes_per_buf).expect("alloc bytestream (serial)");
    let d_one_s = alloc_buffer(&device, bytes_per_buf).expect("alloc one_state (serial)");
    let d_zero_s = alloc_buffer(&device, bytes_per_buf).expect("alloc zero_state (serial)");
    let d_out_s = alloc_buffer(&device, bytes_per_buf).expect("alloc output (serial)");
    d_bytestream_s.copy_from_host(unsafe {
        std::slice::from_raw_parts(bytestream_u32.as_ptr() as *const u8, bytes_per_buf)
    }).expect("h2d bytestream (serial)");
    let one = directx_shader_translate::range_coder::one_state();
    let zero = directx_shader_translate::range_coder::zero_state();
    let one_u32: Vec<u32> = one.iter().map(|&v| v as u32).collect();
    let zero_u32: Vec<u32> = zero.iter().map(|&v| v as u32).collect();
    d_one_s.copy_from_host(unsafe { std::slice::from_raw_parts(one_u32.as_ptr() as *const u8, bytes_per_buf) }).expect("h2d one");
    d_zero_s.copy_from_host(unsafe { std::slice::from_raw_parts(zero_u32.as_ptr() as *const u8, bytes_per_buf) }).expect("h2d zero");

    let serial_cfg = LaunchConfig::linear(1, serial_kernel.local_size.0);
    let serial_spirv: Vec<u8> = serial_kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let serial_compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, serial_kernel.entry_point, serial_spirv);

    // ウォームアップ1回(パイプライン構築コストの影響を減らす)。
    device
        .launch_kernel(
            &serial_compiled,
            &serial_cfg,
            &[
                KernelArg::Ptr(d_bytestream_s.as_ptr()),
                KernelArg::Ptr(d_one_s.as_ptr()),
                KernelArg::Ptr(d_zero_s.as_ptr()),
                KernelArg::Ptr(d_out_s.as_ptr()),
                KernelArg::Usize(PADDED_LEN),
            ],
        )
        .expect("warmup serial");
    device.synchronize().expect("sync warmup serial");

    let t0 = Instant::now();
    for _ in 0..ITERATIONS {
        device
            .launch_kernel(
                &serial_compiled,
                &serial_cfg,
                &[
                    KernelArg::Ptr(d_bytestream_s.as_ptr()),
                    KernelArg::Ptr(d_one_s.as_ptr()),
                    KernelArg::Ptr(d_zero_s.as_ptr()),
                    KernelArg::Ptr(d_out_s.as_ptr()),
                    KernelArg::Usize(PADDED_LEN),
                ],
            )
            .expect("launch serial");
    }
    device.synchronize().expect("sync serial");
    let serial_elapsed = t0.elapsed();

    // --- 32レーン並列版(32シンボル、build_range_decoder_parallel_kernel) ---
    let parallel_kernel = build_range_decoder_parallel_kernel(32);
    let d_bytestream_p = alloc_buffer(&device, bytes_per_buf).expect("alloc bytestream (parallel)");
    let d_zos_p = alloc_buffer(&device, bytes_per_buf * 2).expect("alloc zero_one_state (parallel)");
    let d_ctx_p = alloc_buffer(&device, bytes_per_buf).expect("alloc context_states (parallel)");
    let d_out_p = alloc_buffer(&device, bytes_per_buf).expect("alloc output (parallel)");
    d_bytestream_p.copy_from_host(unsafe {
        std::slice::from_raw_parts(bytestream_u32.as_ptr() as *const u8, bytes_per_buf)
    }).expect("h2d bytestream (parallel)");
    let mut zos_u32 = vec![0u32; PADDED_LEN * 2];
    for i in 0..256 {
        zos_u32[i] = zero[i] as u32;
        zos_u32[256 + i] = one[i] as u32;
    }
    d_zos_p.copy_from_host(unsafe { std::slice::from_raw_parts(zos_u32.as_ptr() as *const u8, bytes_per_buf * 2) }).expect("h2d zos");
    let ctx_init = vec![128u32; PADDED_LEN];
    d_ctx_p.copy_from_host(unsafe { std::slice::from_raw_parts(ctx_init.as_ptr() as *const u8, bytes_per_buf) }).expect("h2d ctx init");

    let parallel_cfg = LaunchConfig::linear(parallel_kernel.local_size.0, parallel_kernel.local_size.0);
    let parallel_spirv: Vec<u8> = parallel_kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let parallel_compiled = CompiledKernel::spirv(OPENCUDA_VULKAN_CHAIN_KERNEL_NAME, parallel_kernel.entry_point, parallel_spirv);

    device
        .launch_kernel(
            &parallel_compiled,
            &parallel_cfg,
            &[
                KernelArg::Ptr(d_bytestream_p.as_ptr()),
                KernelArg::Ptr(d_zos_p.as_ptr()),
                KernelArg::Ptr(d_ctx_p.as_ptr()),
                KernelArg::Ptr(d_out_p.as_ptr()),
                KernelArg::Usize(PADDED_LEN),
            ],
        )
        .expect("warmup parallel");
    device.synchronize().expect("sync warmup parallel");

    let t1 = Instant::now();
    for _ in 0..ITERATIONS {
        device
            .launch_kernel(
                &parallel_compiled,
                &parallel_cfg,
                &[
                    KernelArg::Ptr(d_bytestream_p.as_ptr()),
                    KernelArg::Ptr(d_zos_p.as_ptr()),
                    KernelArg::Ptr(d_ctx_p.as_ptr()),
                    KernelArg::Ptr(d_out_p.as_ptr()),
                    KernelArg::Usize(PADDED_LEN),
                ],
            )
            .expect("launch parallel");
    }
    device.synchronize().expect("sync parallel");
    let parallel_elapsed = t1.elapsed();

    let serial_per_call = serial_elapsed / ITERATIONS;
    let parallel_per_call = parallel_elapsed / ITERATIONS;

    println!(
        "速度比較(1呼び出しあたり、{ITERATIONS}回平均、dispatch_spirvが毎回パイプラインを再構築する実装のためオーバーヘッド込み): \
         逐次版(1invocation, 32シンボル)={serial_per_call:?}, 並列版(32invocation, 32コンテキスト)={parallel_per_call:?}"
    );

    // 正直な開示: dispatch_spirvが呼び出しごとにVkPipeline/DescriptorPool
    // 等を毎回作り直す実装のため、この比較は「実際の32シンボル分の
    // 計算」よりも「パイプライン構築オーバーヘッド」が支配的になる
    // 可能性が高い——結果を数値として報告するのみで、有意な優劣の
    // 主張はしない(単発計測・小サンプルのため統計的な結論には使えない)。
}
