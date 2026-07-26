//! 一時的な調査用ツール。`triangle_vs.dxbc`/`triangle_ps.dxbc`から生成した
//! SPIR-Vバイナリをファイルへ書き出す(`spirv-val`等の外部ツールで
//! クロスチェックするため)。
//! `cargo run -p directx-shader-translate --example dump_graphics_spirv -- vs out.spv`
//! `cargo run -p directx-shader-translate --example dump_graphics_spirv -- ps out.spv`

use directx_shader_translate::spirv_gen::{translate_pixel_shader, translate_vertex_shader};

fn main() {
    let stage = std::env::args().nth(1).expect("usage: dump_graphics_spirv <vs|ps> <out.spv>");
    let out_path = std::env::args().nth(2).expect("usage: dump_graphics_spirv <vs|ps> <out.spv>");

    let words = match stage.as_str() {
        "vs" => {
            let bytes = include_bytes!("../shaders/triangle_vs.dxbc");
            translate_vertex_shader(bytes).expect("translate triangle_vs.dxbc").spirv_words
        }
        "ps" => {
            let bytes = include_bytes!("../shaders/triangle_ps.dxbc");
            translate_pixel_shader(bytes).expect("translate triangle_ps.dxbc").spirv_words
        }
        other => panic!("unknown stage: {other}"),
    };

    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    std::fs::write(&out_path, bytes).expect("write spirv file");
    println!("wrote {} words to {out_path}", words.len());
}
