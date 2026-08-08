//! 実PNGファイルからの`TextureRgba8`読み込み(2026-08-08、2Dスプライト
//! 描画プロトタイプの増分)。車輪の再発明を避け、実績のある`png`クレート
//! (0.17系)のデコーダをそのまま利用する——PNGのチャンク構造/フィルタ/
//! zlib展開等を自前実装することはしない。
//!
//! **対応範囲**: PNGが実際に持ちうる色形式(RGB・RGBA・グレースケール・
//! グレースケール+アルファ・パレット〈インデックスカラー〉)すべてを
//! `png::Transformations::EXPAND`(パレット/グレースケールをRGBへ展開)+
//! `STRIP_16`(16bitチャンネルを8bitへ正規化)で統一的にRGB(A)8bitへ
//! 変換した上で、`TextureRgba8`(常にRGBA8、アルファ無しの入力は
//! `a=255`で埋める)へ揃える。インターレースPNGも`png`クレートが
//! デコード時に自動で展開するため、追加のハンドリングは不要。

use crate::TextureRgba8;
use crate::Rgba8;

#[derive(Debug, thiserror::Error)]
pub enum PngLoadError {
    #[error("PNGデコードエラー: {0}")]
    Decode(#[from] png::DecodingError),
    #[error("対応していないPNGカラータイプ: {0:?}")]
    UnsupportedColorType(png::ColorType),
}

/// PNGバイト列(`std::fs::read`等で読み込んだファイル全体)から
/// `TextureRgba8`をデコードする。
pub fn load_png_rgba8(bytes: &[u8]) -> Result<TextureRgba8, PngLoadError> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    let bytes = &buf[..info.buffer_size()];

    let pixels: Vec<Rgba8> = match info.color_type {
        png::ColorType::Rgba => bytes.chunks_exact(4).map(|c| Rgba8 { r: c[0], g: c[1], b: c[2], a: c[3] }).collect(),
        png::ColorType::Rgb => bytes.chunks_exact(3).map(|c| Rgba8 { r: c[0], g: c[1], b: c[2], a: 255 }).collect(),
        png::ColorType::GrayscaleAlpha => bytes.chunks_exact(2).map(|c| Rgba8 { r: c[0], g: c[0], b: c[0], a: c[1] }).collect(),
        png::ColorType::Grayscale => bytes.iter().map(|&g| Rgba8 { r: g, g, b: g, a: 255 }).collect(),
        other => return Err(PngLoadError::UnsupportedColorType(other)),
    };

    Ok(TextureRgba8 { width: info.width, height: info.height, pixels })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// 既知のRGBA画像を`png`クレートのエンコーダで実際にPNGバイト列へ
    /// エンコードし、`load_png_rgba8`でデコードした結果が完全一致する
    /// ことを確認する(実PNGバイト列を経由した往復検証、フェイクの
    /// アサーションではない)。
    fn encode_test_png(width: u32, height: u32, pixels: &[Rgba8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(Cursor::new(&mut out), width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("write PNG header");
            let mut raw = Vec::with_capacity(pixels.len() * 4);
            for p in pixels {
                raw.extend_from_slice(&[p.r, p.g, p.b, p.a]);
            }
            writer.write_image_data(&raw).expect("write PNG image data");
        }
        out
    }

    #[test]
    fn round_trips_a_known_2x2_rgba_checkerboard_through_real_png_encode_decode() {
        let red = Rgba8 { r: 220, g: 20, b: 20, a: 255 };
        let green = Rgba8 { r: 20, g: 220, b: 20, a: 255 };
        let blue = Rgba8 { r: 20, g: 20, b: 220, a: 255 };
        let yellow = Rgba8 { r: 220, g: 220, b: 20, a: 255 };
        let original = vec![red, green, blue, yellow];

        let png_bytes = encode_test_png(2, 2, &original);
        let texture = load_png_rgba8(&png_bytes).expect("decode real PNG bytes");

        assert_eq!(texture.width, 2);
        assert_eq!(texture.height, 2);
        assert_eq!(texture.pixels, original);
    }

    #[test]
    fn round_trips_semi_transparent_alpha_values_through_real_png_encode_decode() {
        let translucent = Rgba8 { r: 100, g: 150, b: 200, a: 77 };
        let opaque = Rgba8 { r: 5, g: 5, b: 5, a: 255 };
        let original = vec![translucent, opaque];

        let png_bytes = encode_test_png(2, 1, &original);
        let texture = load_png_rgba8(&png_bytes).expect("decode real PNG bytes");

        assert_eq!(texture.pixels, original, "アルファチャンネルを含むPNGの往復でピクセル値が失われている");
    }

    #[test]
    fn honestly_rejects_garbage_bytes_that_are_not_a_png() {
        let garbage = [0u8; 16];
        assert!(load_png_rgba8(&garbage).is_err());
    }
}
