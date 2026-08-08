//! `assets/sample_sprite.png`(2x2、市松模様RGBA)を生成するための
//! 小さなツール(2026-08-08)。このリポジトリのテスト/デモが使う実PNG
//! アセットを、外部の画像ツールに頼らず`png`クレートで直接生成できる
//! ようにするためのもの(再現性のため残置)。
//!
//! 実行方法: `cargo run -p directx-graphics-vulkan --example
//! generate_sample_sprite_png`

use std::io::Cursor;

fn main() {
    let width = 2u32;
    let height = 2u32;
    // row0: 赤, 緑 / row1: 青(半透明), 黄
    let pixels: [[u8; 4]; 4] = [
        [220, 20, 20, 255],
        [20, 220, 20, 255],
        [20, 20, 220, 160],
        [220, 220, 20, 255],
    ];

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut out), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("write PNG header");
        let raw: Vec<u8> = pixels.iter().flatten().copied().collect();
        writer.write_image_data(&raw).expect("write PNG image data");
    }

    let out_path = "crates/directx-graphics-vulkan/assets/sample_sprite.png";
    std::fs::create_dir_all("crates/directx-graphics-vulkan/assets").expect("create assets dir");
    std::fs::write(out_path, &out).expect("write PNG file");
    println!("生成しました: {out_path} ({} bytes)", out.len());
}
