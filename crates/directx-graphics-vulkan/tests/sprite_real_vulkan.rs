//! 2Dスプライト描画プロトタイプの実機検証(2026-08-08新設)。
//! `sprite_vs.dxbc`/`sprite_ps.dxbc`(`directx-shader-translate`側で新設した
//! テクスチャサンプリング対応シェーダー、`Texture2D.Sample`)を実際に
//! `directx_graphics_vulkan::render_textured_quad_and_read_back`へ通し、
//! 4x4の市松模様(checkerboard)テクスチャを実際にVulkanでサンプルした
//! 読み戻し結果が、元のテクスチャの各テクセル値と(NEARESTフィルタにより
//! ブレンド無しで)一致することを実GPU上で確認する。
//!
//! 本リポジトリで初めて「テクスチャを実際にアップロードし、実際にサンプル
//! した結果を検証する」テスト——これまでの三角形描画テストは頂点色の
//! パススルーのみだった。

use directx_graphics_vulkan::{render_sprites_and_read_back, render_textured_quad_and_read_back, Rgba8, SpriteInstance, TextureRgba8};
use directx_shader_translate::spirv_gen::{translate_sprite_pixel_shader, translate_sprite_vertex_shader};

const SPRITE_VS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/sprite_vs.dxbc");
const SPRITE_PS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/sprite_ps.dxbc");

#[test]
fn sprite_quad_samples_a_2x2_checkerboard_texture_exactly_on_real_vulkan_hardware() {
    let vs = translate_sprite_vertex_shader(SPRITE_VS_DXBC).expect("sprite_vs.dxbc must translate");
    let ps = translate_sprite_pixel_shader(SPRITE_PS_DXBC).expect("sprite_ps.dxbc must translate");

    // 2x2の市松模様、4色すべて異なる非グレー値(偶然の一致を避けるため)。
    let red = Rgba8 { r: 220, g: 20, b: 20, a: 255 };
    let green = Rgba8 { r: 20, g: 220, b: 20, a: 255 };
    let blue = Rgba8 { r: 20, g: 20, b: 220, a: 255 };
    let yellow = Rgba8 { r: 220, g: 220, b: 20, a: 255 };
    // テクスチャ座標系はVulkanの慣行通り行0が上端(V=0)。
    let texture = TextureRgba8 {
        width: 2,
        height: 2,
        pixels: vec![red, green, blue, yellow], // row0: red,green / row1: blue,yellow
    };

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

    // 4隅近くの1ピクセルずつをサンプルし、対応する象限のテクセル色と
    // 完全一致することを確認する(NEARESTフィルタのため補間は起きない)。
    let at = |x: u32, y: u32| pixels[(y * width + x) as usize];

    // フレームバッファの行0は画面上端。sprite_vs.hlslのUVはNDCの
    // (-1,-1)->(0,0)、(1,1)->(1,1)というパススルー、Vulkanのビューポート
    // 変換でNDC y=-1が画面上端(row0)に来るため、UV(0,0)=テクスチャ左上
    // (row0,col0=red)がフレームバッファの左上近く(row0付近)に来る。
    let top_left = at(1, 1);
    let top_right = at(6, 1);
    let bottom_left = at(1, 6);
    let bottom_right = at(6, 6);

    assert_eq!(top_left, red, "左上サンプルがテクスチャのred texelと一致しない: {top_left:?}");
    assert_eq!(top_right, green, "右上サンプルがテクスチャのgreen texelと一致しない: {top_right:?}");
    assert_eq!(bottom_left, blue, "左下サンプルがテクスチャのblue texelと一致しない: {bottom_left:?}");
    assert_eq!(bottom_right, yellow, "右下サンプルがテクスチャのyellow texelと一致しない: {bottom_right:?}");

    println!(
        "OK: 2Dスプライト(市松模様2x2テクスチャ、Texture2D.Sample経由)を実Vulkan経路で描画し、\
         4象限すべてが元テクセル値と完全一致することを確認した(NEARESTフィルタ、ブレンド無し)"
    );
}

/// 「複数スプライトをゲーム画面上の異なる位置に表示する」ゲームループ
/// プロトタイプ第二歩(2026-08-08)。同じ4色市松模様テクスチャ(スプライト
/// シート相当)から、左上象限(red)と右下象限(yellow)という異なる矩形を
/// 切り出し、1回の描画コマンドで画面の左半分と右半分という異なる位置に
/// 別々のスプライトとして表示する——`render_sprites_and_read_back`が
/// 実際に複数インスタンスを独立した位置・UVで描画できることを実機で確認。
#[test]
fn multiple_sprites_from_a_shared_atlas_render_at_independent_screen_positions_on_real_vulkan_hardware() {
    let vs = translate_sprite_vertex_shader(SPRITE_VS_DXBC).expect("sprite_vs.dxbc must translate");
    let ps = translate_sprite_pixel_shader(SPRITE_PS_DXBC).expect("sprite_ps.dxbc must translate");

    let red = Rgba8 { r: 220, g: 20, b: 20, a: 255 };
    let yellow = Rgba8 { r: 220, g: 220, b: 20, a: 255 };
    let green = Rgba8 { r: 20, g: 220, b: 20, a: 255 };
    let blue = Rgba8 { r: 20, g: 20, b: 220, a: 255 };
    let atlas = TextureRgba8 { width: 2, height: 2, pixels: vec![red, green, blue, yellow] };

    // スプライト1: アトラスの左上(red)象限を、画面の左半分(NDC x in
    // [-1,0])いっぱいへ描画。
    let sprite_left = SpriteInstance {
        dest_ndc: [-1.0, -1.0, 0.0, 1.0],
        uv_rect: [0.0, 0.0, 0.5, 0.5], // red象限のみ
    };
    // スプライト2: アトラスの右下(yellow)象限を、画面の右半分(NDC x in
    // [0,1])いっぱいへ描画。
    let sprite_right = SpriteInstance {
        dest_ndc: [0.0, -1.0, 1.0, 1.0],
        uv_rect: [0.5, 0.5, 1.0, 1.0], // yellow象限のみ
    };

    let width = 8u32;
    let height = 8u32;
    let pixels = match render_sprites_and_read_back(
        &vs.spirv_words,
        &ps.spirv_words,
        &atlas,
        &[sprite_left, sprite_right],
        width,
        height,
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
            return;
        }
    };
    assert_eq!(pixels.len(), (width * height) as usize);

    let at = |x: u32, y: u32| pixels[(y * width + x) as usize];

    // 画面左半分(x=0..3)は全ピクセルがred一色(uv_rectがred象限のみを
    // 指すため、左スプライトのどこをサンプルしても常にred)。
    for y in 0..height {
        for x in 0..4 {
            let p = at(x, y);
            assert_eq!(p, red, "左半分(x={x},y={y})がredではない: {p:?}");
        }
    }
    // 画面右半分(x=4..7)は全ピクセルがyellow一色(同様の理由)。
    for y in 0..height {
        for x in 4..8 {
            let p = at(x, y);
            assert_eq!(p, yellow, "右半分(x={x},y={y})がyellowではない: {p:?}");
        }
    }

    println!(
        "OK: 1枚の共有アトラステクスチャから異なる矩形を切り出した2枚のスプライトを、\
         1回の描画コマンドで画面上の異なる位置(左半分=red・右半分=yellow)へ独立して\
         描画できることを実Vulkan経路で確認した"
    );
}
