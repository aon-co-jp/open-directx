//! FFv1本体の**ピクセル処理ループ**(近傍差分からコンテキストインデックス
//! を選び、`put_symbol`/`get_symbol`で予測誤差を符号化する部分)。
//!
//! 2026-09-13、実FFmpegソース(`libavcodec/ffv1_template.c`の
//! `predict`/`get_context`、`libavcodec/ffv1enc.c`の`quant11`/`quant5`
//! テーブル、いずれも本セッションで実際にfetchして確認)を精読し、
//! 当初の簡略版(2勾配・`clamp`量子化)から、**実際のFFv1の予測式・
//! コンテキスト式・既定量子化テーブルそのもの**へアップグレードした。
//!
//! 実際にfetchして確認した`get_context`(`ffv1_template.c`より抜粋、
//! 変数名もそのまま): `L`=left、`LT`=topleft、`T`=top、`RT`=topright、
//! `LL`=2つ左、`TT`=2つ上。
//! ```c
//! quant_table[0][(L-LT)&255] + quant_table[1][(LT-T)&255] +
//! quant_table[2][(T-RT)&255] + quant_table[3][(LL-L)&255] +
//! quant_table[4][(TT-T)&255]
//! ```
//! `quant_table[0]`/`[1]`は`quant11`、`quant_table[2..5]`は`quant5`
//! (`ffv1enc.c`の実際のテーブル割り当てを確認——当初`[2]`も`quant11`だと
//! 誤解しており、`.mkv`実バイナリ互換調査の過程で修正した、後述)。
//! `predict`(同ファイル)は`mid_pred(L, L+T-LT, T)`——3値の中央値であり、
//! これは本プロジェクトが既に実装しているMED予測器(`spirv_gen`/
//! `med2d`)と数式として完全に一致することを、実ソースを読んで
//! 確認できた。
//!
//! これまでに実装済みの3つの層——MED予測器、レンジコーダー本体
//! `get_rac`(実GT730ハードウェアでビット単位検証済み)、FFv1の
//! シンボル符号化層`get_symbol`/`put_symbol`(RFC 9043 Figure 21、
//! 往復検証済み)——を、この実FFv1コンテキスト式で結合する。
//!
//! **正直な開示(それでもなお`.mkv`とバイナリ互換ではない理由)**:
//! - 実際のFFv1ビットストリームのコンテナ形式(Matroska/AVI等でのEBML
//!   マキシング、FFv1のフレーム/スライスヘッダのビット単位レイアウト
//!   〈version, coder_type, bits_per_raw_sample等〉、末尾のCRC32
//!   チェックサム)は実装していない——出力は`RangeEncoderCpu`の生
//!   出力そのもの。
//! - RGBのJPEG2000-RCT可逆カラー変換は未実装(単一プレーン=グレー
//!   スケール相当のみ)。
//! - `quant_table_count==5`(大コンテキスト、本モジュールが実装する形)
//!   と`==3`(小コンテキスト)の切り替えロジック、カスタム量子化
//!   テーブルのビットストリーム格納(`ff_ffv1_write_quant_tables`)は
//!   未実装——常に大コンテキスト・既定テーブル固定。
//! - 初期状態(`initial_states`)は全コンテキスト`128`固定(実FFv1は
//!   バージョン2以降`ver2_state`という既定の非一様初期値を使う)。
//!
//! これらが揃って初めて、実際のffmpegが吐く`.mkv`ファイルとバイト単位
//! で相互運用できる——このモジュールは「予測式・コンテキスト式・量子化
//! テーブルは実物」だが「コンテナ/ヘッダ/チェックサムは無し」という
//! 段階であり、次の現実的な一歩は上記いずれか(特にスライスヘッダの
//! ビット単位レイアウト)を追加することだと記録しておく。

use crate::range_coder::{get_symbol, put_symbol, RangeDecoderCpu, RangeEncoderCpu};

/// MED(median edge detector)予測器。FFmpeg実ソース
/// (`ffv1_template.c`の`predict`: `mid_pred(L, L+T-LT, T)`)と数式として
/// 一致することを確認済み。`spirv_gen`/`med2d`のGPU版と同じ式。
fn med_predict(left: i32, top: i32, topleft: i32) -> i32 {
    if topleft >= left.max(top) {
        left.min(top)
    } else if topleft <= left.min(top) {
        left.max(top)
    } else {
        left + top - topleft
    }
}

/// FFmpeg実ソース(`ffv1enc.c`)の`quant11`テーブルをそのまま転記した
/// もの(8bit版、非10bit版)。差分`d`を`d&255`でインデックスし、
/// `-5..=5`の11段階へ量子化する。
#[rustfmt::skip]
const QUANT11: [i8; 256] = [
     0,  1,  2,  2,  2,  3,  3,  3,  3,  3,  3,  3,  4,  4,  4,  4,
     4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,  4,
     4,  4,  4,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,
     5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,
     5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,
     5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,
     5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,
     5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,  5,
    -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5,
    -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5,
    -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5,
    -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5,
    -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5,
    -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -5, -4, -4,
    -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4,
    -4, -4, -4, -4, -4, -3, -3, -3, -3, -3, -3, -3, -2, -2, -2, -1,
];

/// FFmpeg実ソース(`ffv1enc.c`)の`quant5`テーブルをそのまま転記した
/// もの(8bit版)。`-2..=2`の5段階へ量子化する。
#[rustfmt::skip]
const QUANT5: [i8; 256] = [
     0,  1,  1,  1,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,
    -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -2, -1, -1, -1,
];

fn q11(d: i32) -> i32 {
    QUANT11[(d & 0xFF) as usize] as i32
}

fn q5(d: i32) -> i32 {
    QUANT5[(d & 0xFF) as usize] as i32
}

/// 画像内の`(x, y)`の近傍6点(`left`/`top`/`topleft`/`topright`/
/// `ll`=2つ左/`tt`=2つ上)を、因果的な(まだ復号済みのピクセルのみを
/// 参照する)フォールバック連鎖で取得する。
///
/// **重要(バグ修正、2026-09-13)**: 当初`med2d.rs`と同じ「境界は`center`
/// (自分自身のピクセル値)で埋める」簡略化を流用したが、これは**復号側
/// では成立しない**——デコード側は「そのピクセル自身の値」こそがこれ
/// から復号しようとしている未知の値であり、参照すると未初期化の
/// プレースホルダ`0`を読んでエンコード時とズレる(実際にこのテストで
/// 最初の実装がこの理由で全滅して発見した)。既に復号済みの近傍だけを
/// 使うフォールバック連鎖に修正済み。
fn neighbors(image: &[i32], width: i32, x: i32, y: i32) -> (i32, i32, i32, i32, i32, i32) {
    let get = |xx: i32, yy: i32| -> Option<i32> {
        if xx >= 0 && xx < width && yy >= 0 { Some(image[(yy * width + xx) as usize]) } else { None }
    };
    let raw_left = get(x - 1, y);
    let raw_top = get(x, y - 1);
    let raw_topleft = get(x - 1, y - 1);
    let raw_topright = get(x + 1, y - 1);
    let raw_ll = get(x - 2, y);
    let raw_tt = get(x, y - 2);

    let left = raw_left.or(raw_top).unwrap_or(0);
    let top = raw_top.or(raw_left).unwrap_or(0);
    let topleft = raw_topleft.or(raw_left).or(raw_top).unwrap_or(0);
    let topright = raw_topright.or(raw_top).unwrap_or(top);
    let ll = raw_ll.or(raw_left).unwrap_or(left);
    let tt = raw_tt.or(raw_top).unwrap_or(top);
    (left, top, topleft, topright, ll, tt)
}

/// 実FFv1の`get_context`(`ffv1_template.c`、このモジュール冒頭の
/// docコメント参照)をそのまま実装する。符号が負の場合はインデックスを
/// 反転し、符号化する差分の符号も反転する(FFv1の「コンテキストの
/// 符号対称性による状態テーブル半減」規約)。
///
/// **バグ修正(2026-09-13、続き)**: `.mkv`実バイナリ互換を目指して
/// RFC 9043のParameters()/QuantizationTableSet()擬似コードを精読した
/// ところ、`ffv1enc.c`の実際のテーブル割り当て
/// (`quant_tables[1][0]=quant11`, `[1]=11*quant11`, `[2]=121*quant5`,
/// `[3]=605*quant5`, `[4]=3025*quant5`)を見落としていたことが判明した
/// ——**3項目(`top-topright`)は`quant11`ではなく`quant5`が正しい**
/// (0,1番目のみ`quant11`、2,3,4番目は`quant5`)。これに伴い
/// `CONTEXT_COUNT`も、`QuantizationTableSet`の`scale`累積式
/// (`scale *= 2*len_count[i][j]-1`、`quant11`の`len_count=6`→11倍、
/// `quant5`の`len_count=3`→5倍)で正しく再計算した値
/// (`1→11→121→605→3025→15125`、`context_count=ceil(15125/2)=7563`)
/// へ修正した——以前の`16638`は誤って全5項目`quant11`だと仮定した
/// 計算だった。
fn compute_context(left: i32, top: i32, topleft: i32, topright: i32, ll: i32, tt: i32) -> (usize, bool) {
    let ctx = q11(left - topleft) + 11 * q11(topleft - top) + 121 * q5(top - topright) + 605 * q5(ll - left) + 3025 * q5(tt - top);
    if ctx < 0 {
        ((-ctx) as usize, true)
    } else {
        (ctx as usize, false)
    }
}

/// [`compute_context`]が返し得るコンテキストインデックスの最大値+1。
/// RFC 9043の`QuantizationTableSet`が実際に計算する
/// `context_count[i] = ceil(scale/2)`(`scale`は`quant11`×2項+
/// `quant5`×3項の`len_count`から`1→11→121→605→3025→15125`と累積、
/// 上の`compute_context`のdocコメント参照)。
const CONTEXT_COUNT: usize = 7563;

/// 画像(行優先、`i32`ピクセル値)を、実FFv1の予測式・コンテキスト式・
/// 既定量子化テーブルで可逆圧縮する。
pub fn encode_plane(image: &[i32], width: u32, height: u32) -> Vec<u8> {
    let w = width as i32;
    let h = height as i32;
    debug_assert_eq!(image.len(), (w * h) as usize);

    let mut enc = RangeEncoderCpu::new();
    let mut states = vec![[128u8; 32]; CONTEXT_COUNT];

    for y in 0..h {
        for x in 0..w {
            let (left, top, topleft, topright, ll, tt) = neighbors(image, w, x, y);
            let pred = med_predict(left, top, topleft);
            let actual = image[(y * w + x) as usize];
            let diff = actual - pred;

            let (ctx, flip) = compute_context(left, top, topleft, topright, ll, tt);
            let v = if flip { -diff } else { diff };
            put_symbol(&mut enc, &mut states[ctx], v, true);
        }
    }

    enc.finish()
}

/// [`encode_plane`]の逆操作。`width`/`height`は呼び出し側が(実際の
/// FFv1ならスライスヘッダから)知っている前提——このモジュールは
/// ヘッダを符号化しない(冒頭のdocコメント参照)。
pub fn decode_plane(bytes: &[u8], width: u32, height: u32) -> Vec<i32> {
    let w = width as i32;
    let h = height as i32;

    // `RangeDecoderCpu::new`は先頭2バイトを読むため、短い入力でも
    // 安全なようパディングする(range_coder.rsのテストと同じ理由)。
    let mut padded = bytes.to_vec();
    padded.extend_from_slice(&[0, 0, 0, 0]);

    let mut dec = RangeDecoderCpu::new(&padded);
    let mut states = vec![[128u8; 32]; CONTEXT_COUNT];
    let mut image = vec![0i32; (w * h) as usize];

    for y in 0..h {
        for x in 0..w {
            let (left, top, topleft, topright, ll, tt) = neighbors(&image, w, x, y);
            let pred = med_predict(left, top, topleft);

            let (ctx, flip) = compute_context(left, top, topleft, topright, ll, tt);
            let v = get_symbol(&mut dec, &mut states[ctx], true);
            let diff = if flip { -v } else { v };

            image[(y * w + x) as usize] = pred + diff;
        }
    }

    image
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(width: u32, height: u32) -> Vec<i32> {
        // 単純な勾配ではなく、MEDの3分岐すべてを実際に踏むよう
        // 疑似乱数的なパターン+局所的な滑らかさを混ぜる
        // (`med_predictor_real_vulkan.rs`と同じ配慮)。
        (0..(width * height))
            .map(|i| {
                let x = i % width;
                let y = i / width;
                (((x * 17 + y * 31) % 200) as i32) + ((x as i32 * 3 - y as i32 * 5) % 7)
            })
            .collect()
    }

    #[test]
    fn encode_then_decode_reproduces_the_original_image_exactly() {
        const WIDTH: u32 = 20;
        const HEIGHT: u32 = 15;
        let image = make_test_image(WIDTH, HEIGHT);

        let compressed = encode_plane(&image, WIDTH, HEIGHT);
        let decoded = decode_plane(&compressed, WIDTH, HEIGHT);

        assert_eq!(decoded, image, "MED予測+コンテキスト選択+get_symbol/put_symbolの往復で画像が完全に再現されるべき");

        // 参考情報として圧縮率を出力する(このテスト自体の合否には
        // 使わない——実FFv1既定テーブルを使ってはいるが、スライス
        // ヘッダ等が無い簡易フォーマットのため圧縮率の最適性は
        // 主張しない、正しさの検証のみが目的)。
        println!(
            "OK: {WIDTH}x{HEIGHT}={}ピクセルの画像が可逆圧縮・復元で完全一致(圧縮後: {}バイト、生データ: {}バイト)",
            WIDTH * HEIGHT,
            compressed.len(),
            WIDTH as usize * HEIGHT as usize * std::mem::size_of::<i32>()
        );
    }

    #[test]
    fn encode_then_decode_handles_a_flat_solid_color_image() {
        // 全ピクセル同値(差分が常に0になる、極端なケース)。
        const WIDTH: u32 = 10;
        const HEIGHT: u32 = 10;
        let image = vec![42i32; (WIDTH * HEIGHT) as usize];

        let compressed = encode_plane(&image, WIDTH, HEIGHT);
        let decoded = decode_plane(&compressed, WIDTH, HEIGHT);

        assert_eq!(decoded, image);
    }

    #[test]
    fn encode_then_decode_handles_negative_and_large_pixel_values() {
        // 符号付き差分の往復(負値・大きな値)も実際に踏む。
        const WIDTH: u32 = 12;
        const HEIGHT: u32 = 8;
        let image: Vec<i32> = (0..(WIDTH * HEIGHT))
            .map(|i| {
                let x = (i % WIDTH) as i32;
                let y = (i / WIDTH) as i32;
                -1000 + x * x - y * 50
            })
            .collect();

        let compressed = encode_plane(&image, WIDTH, HEIGHT);
        let decoded = decode_plane(&compressed, WIDTH, HEIGHT);

        assert_eq!(decoded, image);
    }

    #[test]
    fn quant_tables_match_the_real_ffmpeg_source_at_key_sample_points() {
        // ffv1enc.cから実際に転記した値であることの回帰検知
        // (書き写しミスがあれば即座に検出できるよう、代表的な数点を
        // 独立に確認する)。
        assert_eq!(QUANT11[0], 0);
        assert_eq!(QUANT11[1], 1);
        assert_eq!(QUANT11[255], -1);
        assert_eq!(QUANT11[128], -5);
        assert_eq!(QUANT5[0], 0);
        assert_eq!(QUANT5[255], -1);
        assert_eq!(QUANT5[128], -2);
    }
}
