//! FFv1本体の**ピクセル処理ループ**(近傍差分からコンテキストインデックス
//! を選び、`put_symbol`/`get_symbol`で予測誤差を符号化する部分)、
//! 簡略版の実装(2026-09-13追加)。
//!
//! これまでに実装済みの3つの層——MED予測器(`spirv_gen`/`med2d`、GPU側)、
//! レンジコーダー本体`get_rac`(`range_coder`、実GT730ハードウェアで
//! ビット単位検証済み)、FFv1のシンボル符号化層`get_symbol`/`put_symbol`
//! (RFC 9043 Figure 21、往復検証済み)——を実際に1つのピクセルループへ
//! 組み合わせ、**1枚の画像を最後まで可逆圧縮・復元できる**ことを検証する
//! (これまでは各層を個別に検証していたが、実際にFFv1が行う「予測→
//! 誤差計算→コンテキスト選択→シンボル符号化」という一連の流れを通しで
//! 動かすのはこのモジュールが初めて)。
//!
//! **正直な開示(簡略化した点)**:
//! - コンテキストは`left-topleft`と`top-topright`の2勾配のみ使う
//!   (実際のFFv1仕様は最大5勾配——`ll-l2l`/`tr-t2r`も含む——を使う、
//!   量子化テーブルもビット深度・バージョン依存の複雑な既定値がある)。
//! - 量子化関数[`quantize`]は単純な`clamp`(11段階、-5..=5)であり、
//!   実際のFFv1既定量子化テーブル(非線形)ではない。
//! - スライスヘッダ・バージョンフィールド等、実際のFFv1ビットストリーム
//!   コンテナ形式(`av_read_frame`等で読めるファイル形式)は実装していない
//!   ——このモジュールが出力するバイト列は`RangeEncoderCpu`の生出力
//!   そのものであり、実際のFFv1ファイル(`.mkv`等)とは互換性が無い。
//! - 境界(画像端)は`med2d`と同じ「`center`(自分自身のピクセル値)で
//!   埋める」簡略化。
//!
//! これらの簡略化は、「近傍差分→コンテキスト選択→シンボル符号化」という
//! **実際のFFv1のアルゴリズム構造**を検証するには十分であり、実際の
//! `.mkv`ファイルとのバイナリ互換性を主張するものではない。

use crate::range_coder::{get_symbol, put_symbol, RangeDecoderCpu, RangeEncoderCpu};

/// MED(median edge detector)予測器。`spirv_gen`/`med2d`のGPU版・
/// テストのCPU参照実装と同じ式(整数版)。
fn med_predict(left: i32, top: i32, topleft: i32) -> i32 {
    if topleft >= left.max(top) {
        left.min(top)
    } else if topleft <= left.min(top) {
        left.max(top)
    } else {
        left + top - topleft
    }
}

/// 差分を11段階(`-5..=5`)へ量子化する、簡略化した量子化関数
/// (このモジュール冒頭のdocコメント参照——実際のFFv1既定量子化
/// テーブルではない)。
fn quantize(d: i32) -> i32 {
    d.clamp(-5, 5)
}

/// 画像内の`(x, y)`から、MED予測に使う`(left, top, topleft, topright)`
/// を取得する。
///
/// **重要(バグ修正、2026-09-13)**: 当初`med2d.rs`と同じ「境界は`center`
/// (自分自身のピクセル値)で埋める」簡略化を流用したが、これは**復号側
/// では成立しない**——エンコード側は画像全体を既に持っているため
/// `center`(=これから符号化するピクセル自身の値)を参照できるが、
/// デコード側は「そのピクセル自身の値」こそがこれから復号しようとして
/// いる未知の値であり、参照できるはずがない(参照すると常に未初期化の
/// プレースホルダ`0`を読んでしまい、エンコード時とズレて往復が壊れる
/// ——実際にこのテストで最初の実装がこの理由で全滅したことで発見した)。
/// 正しい実装は、**既に復号済みの近傍だけ**を使った因果的な
/// フォールバック連鎖(存在しなければ`left`↔`top`で補い合い、両方
/// 無ければ`0`)にする必要がある——現在の値を参照する`med2d.rs`の
/// 簡略化は「予測のみ」(往復復号ではない)用途では問題にならないが、
/// 実際に符号化→復号のループを組む場合は本関数のように因果性を
/// 保つ必要がある、という教訓をここに記録する。
fn neighbors(image: &[i32], width: i32, x: i32, y: i32) -> (i32, i32, i32, i32) {
    let get = |xx: i32, yy: i32| -> Option<i32> {
        if xx >= 0 && xx < width && yy >= 0 { Some(image[(yy * width + xx) as usize]) } else { None }
    };
    let raw_left = get(x - 1, y);
    let raw_top = get(x, y - 1);
    let raw_topleft = get(x - 1, y - 1);
    let raw_topright = get(x + 1, y - 1);

    let left = raw_left.or(raw_top).unwrap_or(0);
    let top = raw_top.or(raw_left).unwrap_or(0);
    let topleft = raw_topleft.or(raw_left).or(raw_top).unwrap_or(0);
    let topright = raw_topright.or(raw_top).unwrap_or(top);
    (left, top, topleft, topright)
}

/// コンテキストインデックスと、符号を反転させるべきかどうかのフラグを
/// 求める(FFv1の「コンテキストの符号対称性による表引き半減」規約:
/// `quantize(left-topleft)*11+quantize(top-topright)`が負ならインデックス
/// を`-ctx`に、符号化する差分の符号を反転する)。
fn compute_context(left: i32, top: i32, topleft: i32, topright: i32) -> (usize, bool) {
    let ctx = quantize(left - topleft) * 11 + quantize(top - topright);
    if ctx < 0 {
        ((-ctx) as usize, true)
    } else {
        (ctx as usize, false)
    }
}

/// [`compute_context`]が返し得るコンテキストインデックスの個数
/// (`ctx`の絶対値の最大は`5*11+5=60`)。
const CONTEXT_COUNT: usize = 61;

/// 画像(行優先、`i32`ピクセル値、任意のビット深度を想定)を、MED予測+
/// 2勾配コンテキスト+`put_symbol`で可逆圧縮する。
pub fn encode_plane(image: &[i32], width: u32, height: u32) -> Vec<u8> {
    let w = width as i32;
    let h = height as i32;
    debug_assert_eq!(image.len(), (w * h) as usize);

    let mut enc = RangeEncoderCpu::new();
    let mut states = vec![[128u8; 32]; CONTEXT_COUNT];

    for y in 0..h {
        for x in 0..w {
            let (left, top, topleft, topright) = neighbors(image, w, x, y);
            let pred = med_predict(left, top, topleft);
            let actual = image[(y * w + x) as usize];
            let diff = actual - pred;

            let (ctx, flip) = compute_context(left, top, topleft, topright);
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
            let (left, top, topleft, topright) = neighbors(&image, w, x, y);
            let pred = med_predict(left, top, topleft);

            let (ctx, flip) = compute_context(left, top, topleft, topright);
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
        // 使わない——`quantize`/コンテキスト設計が簡略化されているため
        // 圧縮率の最適性は主張しない、正しさの検証のみが目的)。
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
}
