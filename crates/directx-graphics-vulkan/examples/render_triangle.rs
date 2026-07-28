//! 2026-07-27追記(使いやすさ改善): 新しく触る人が「まず動かして見てみる」
//! ための最小デモ。`tests/triangle_real_vulkan.rs`が使っているのと同じ
//! 実DXBC(fxc.exeコンパイル済み)→SPIR-V変換済みシェーダーで、実Vulkan
//! ハードウェア上に実際に三角形を描画し、読み戻したフレームバッファを
//! PPM(P6、追加の画像クレート依存無しで書ける最小フォーマット)として
//! 保存する。ビューア例: `magick render_triangle.ppm render_triangle.png`
//! (ImageMagick)や、多くの画像ビューアはPPMをそのまま開ける。
//!
//! 実行方法:
//! ```bash
//! cargo run -p directx-graphics-vulkan --example render_triangle
//! ```
//!
//! 実Vulkanデバイス/ドライバが無い環境では、このリポジトリの他の
//! 実機テストと同じ方針でエラーメッセージを出して終了する
//! (モックで「成功したふり」はしない)。

use directx_graphics_vulkan::render_gradient_triangle_and_read_back;
use directx_shader_translate::spirv_gen::{translate_pixel_shader, translate_vertex_shader};

const TRIANGLE_VS_DXBC: &[u8] =
    include_bytes!("../../directx-shader-translate/shaders/triangle_vs.dxbc");
const TRIANGLE_PS_DXBC: &[u8] =
    include_bytes!("../../directx-shader-translate/shaders/triangle_ps.dxbc");

fn main() {
    let vs = translate_vertex_shader(TRIANGLE_VS_DXBC).expect("triangle_vs.dxbc must translate");
    let ps = translate_pixel_shader(TRIANGLE_PS_DXBC).expect("triangle_ps.dxbc must translate");

    // 頂点ごとに異なる色(赤/緑/青)を与え、ラスタライザの重心座標補間を
    // 目視でも確認できるグラデーション三角形にする。
    let vertex_colors = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    ];
    let (width, height) = (256u32, 256u32);

    let pixels = match render_gradient_triangle_and_read_back(
        &vs.spirv_words,
        &ps.spirv_words,
        vertex_colors,
        width,
        height,
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "実Vulkanデバイス/ドライバが見つからないため描画できませんでした: {e}\n\
                 (このデモは実機描画専用で、モックでの「成功したふり」はしません)"
            );
            std::process::exit(1);
        }
    };

    let out_path = "render_triangle.ppm";
    let mut body = format!("P6\n{width} {height}\n255\n").into_bytes();
    for px in &pixels {
        body.push(px.r);
        body.push(px.g);
        body.push(px.b);
    }
    std::fs::write(out_path, &body).expect("failed to write render_triangle.ppm");

    println!("描画成功: {width}x{height}のグラデーション三角形を {out_path} に保存しました。");
    println!("(PPM形式。表示するには例えば `magick {out_path} render_triangle.png` などで変換してください)");
}
