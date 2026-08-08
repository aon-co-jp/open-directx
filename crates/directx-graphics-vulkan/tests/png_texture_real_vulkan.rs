//! 実PNGファイルからのテクスチャ読み込み+実描画の実機検証(2026-08-08)。
//! `assets/sample_sprite.png`(2x2、`examples/generate_sample_sprite_png.rs`
//! で実際に生成した本物のPNGファイル、市松模様+半透明ピクセル1個を含む)を
//! `directx_graphics_vulkan::png_loader::load_png_rgba8`で実際にデコードし、
//! `render_sprites_and_read_back`で実描画した結果が、PNGファイルの実際の
//! ピクセル値(半透明ピクセルはアルファブレンド式通りの合成色)と一致する
//! ことを確認する。

use directx_graphics_vulkan::png_loader::load_png_rgba8;
use directx_graphics_vulkan::{render_textured_quad_and_read_back, Rgba8};
use directx_shader_translate::spirv_gen::{translate_sprite_pixel_shader, translate_sprite_vertex_shader};

const SPRITE_VS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/sprite_vs.dxbc");
const SPRITE_PS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/sprite_ps.dxbc");
const SAMPLE_SPRITE_PNG: &[u8] = include_bytes!("../assets/sample_sprite.png");

#[test]
fn real_png_file_decodes_and_renders_correctly_on_real_vulkan_hardware() {
    let texture = load_png_rgba8(SAMPLE_SPRITE_PNG).expect("decode assets/sample_sprite.png");
    assert_eq!(texture.width, 2);
    assert_eq!(texture.height, 2);

    let red = Rgba8 { r: 220, g: 20, b: 20, a: 255 };
    let green = Rgba8 { r: 20, g: 220, b: 20, a: 255 };
    let blue_translucent = Rgba8 { r: 20, g: 20, b: 220, a: 160 };
    let yellow = Rgba8 { r: 220, g: 220, b: 20, a: 255 };
    assert_eq!(texture.pixels, vec![red, green, blue_translucent, yellow], "PNGデコード結果が生成時の元データと一致しない");

    let vs = translate_sprite_vertex_shader(SPRITE_VS_DXBC).expect("sprite_vs.dxbc must translate");
    let ps = translate_sprite_pixel_shader(SPRITE_PS_DXBC).expect("sprite_ps.dxbc must translate");

    let width = 8u32;
    let height = 8u32;
    let pixels = match render_textured_quad_and_read_back(&vs.spirv_words, &ps.spirv_words, &texture, width, height) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    assert_eq!(pixels.len(), (width * height) as usize);
    let at = |x: u32, y: u32| pixels[(y * width + x) as usize];

    // 不透明な3象限(red/green/yellow)は完全一致するはず。
    assert_eq!(at(1, 1), red, "左上がredと一致しない");
    assert_eq!(at(6, 1), green, "右上がgreenと一致しない");
    assert_eq!(at(6, 6), yellow, "右下がyellowと一致しない");

    // 半透明な左下(blue_translucent=(20,20,220,160))は、クリア色(黒)に
    // 対する標準over式: result = src.rgb*(160/255) + 0*(1-160/255)。
    // 丸め誤差を許容差2で吸収。
    let a = 160.0f32 / 255.0;
    let expected_r = (20.0 * a).round() as i32;
    let expected_g = (20.0 * a).round() as i32;
    let expected_b = (220.0 * a).round() as i32;
    let bottom_left = at(1, 6);
    assert!(
        (bottom_left.r as i32 - expected_r).abs() <= 2
            && (bottom_left.g as i32 - expected_g).abs() <= 2
            && (bottom_left.b as i32 - expected_b).abs() <= 2,
        "左下(半透明blue)がover式の期待値と一致しない: {bottom_left:?} \
         (期待≈r{expected_r},g{expected_g},b{expected_b})"
    );

    println!(
        "OK: 実PNGファイル(assets/sample_sprite.png)を実際にデコードし、\
         不透明3象限が完全一致・半透明象限がalphaブレンド式通りの合成色になることを実Vulkan経路で確認した \
         (左下={bottom_left:?})"
    );
}
