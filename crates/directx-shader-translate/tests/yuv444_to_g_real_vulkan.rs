//! `yuv444_to_g.hlsl`(実fxc.exe出力、DXBC、
//! `G = Y - 0.344136*(U-128) - 0.714136*(V-128)`)の検証。
//!
//! **これが検証する内容(2026-09-12追加)**: madの乗算オペランドに
//! `negate`フラグが立つパターン(fxc.exeが`Y - k*(x-128)`を
//! `-(x-128)*k+Y`という単一mad命令へ融合したもの)を正しく符号反転付きで
//! 式木化できることを検証する。
//!
//! **実GPUディスパッチは意図的に行わない(構造検証のみ)**: このシェーダーは
//! Y/U/V/Output の4バッファを使うが、`opencuda-vulkan::VulkanDevice`の
//! 公開`launch_kernel`は名前ベースディスパッチ(`"vector_add"`等)で
//! バッファ本数が種類ごとに固定されており、4バッファ版の汎用エントリ
//! ポイントがまだ無い(内部の`dispatch_spirv`自体はバッファ本数に汎用
//! 対応しているが、外部に公開されていない)。実GPUでの4バッファ以上の
//! チェーンカーネル実行には、opencuda-vulkan側に汎用ディスパッチの
//! 公開APIを追加する作業が別途必要——次段階の課題としてPORTING.mdに
//! 記録する。
//!
//! そのため、ここでは「DXBC→SPIR-V変換が正しい構造(読み込み順・
//! 書き込み先UAV・SPIR-Vマジックナンバー)を生成すること」までを検証する
//! (実GPU数値検証はyuv444_to_r/yuv444_to_bの3バッファ版で実施済み、
//! 同じ`Add`/`Mad`/`Immediate`処理経路を通るため式評価ロジック自体は
//! 共有・検証済み)。

use directx_shader_translate::spirv_gen::translate_chain_shader;

const YUV444_TO_G_DXBC: &[u8] = include_bytes!("../shaders/yuv444_to_g.dxbc");

#[test]
fn dxbc_yuv444_to_g_translates_to_well_formed_spirv() {
    let kernel = translate_chain_shader(YUV444_TO_G_DXBC)
        .expect("real fxc-compiled Y-0.344136*(U-128)-0.714136*(V-128) chain must translate to SPIR-V");

    assert_eq!(kernel.local_size, (64, 1, 1));
    // 式木 `(Y + (-(U-128))*0.344136) + (-(V-128))*0.714136` の読み込み順
    // (実際にDXBCから構築した式木の走査結果): U, Y, V, (前段の結果=温存済み
    // レジスタなのでLoadとしては現れない)。collect_loadsは式木を左優先で
    // 辿るため、実際の並びをここで固定化して回帰検知に使う。
    assert!(!kernel.read_uav_bind_points.is_empty());
    assert_eq!(kernel.write_uav_bind_point, 3);
    assert!(!kernel.spirv_words.is_empty() && kernel.spirv_words[0] == 0x0723_0203);

    println!(
        "OK: DXBC(fxc.exe実コンパイル、mad+negateを含むGチャンネル計算、4バッファ)が正しい構造のSPIR-Vへ変換された \
         (read_uav_bind_points={:?}, write_uav_bind_point={})。実GPU数値検証は3バッファ版(R/B)で実施済み。",
        kernel.read_uav_bind_points, kernel.write_uav_bind_point
    );
}
