//! 一時的な調査用ツール(2026-09-13): 実ffmpegが吐く実FFv1(version 0)
//! パケットのバイト列と、`plane_codec::encode_plane`の出力を直接
//! 比較し、実際にどこまでバイト単位互換に近いかを調べる。
//! `cargo run -p directx-shader-translate --example ffv1_interop_probe`

use directx_shader_translate::plane_codec::encode_plane;

fn main() {
    let image = vec![128i32; 16 * 16];
    let ours = encode_plane(&image, 16, 16);
    print!("ours ({} bytes): ", ours.len());
    for b in &ours {
        print!("{b:02x}");
    }
    println!();

    let real_ffmpeg_hex = "f2fc16c606e5c3a22afa57dd82667c34cd9a0007a4afff9fffffe";
    println!("real ffmpeg (level 0, 16x16 gray=128): {real_ffmpeg_hex}");
}
