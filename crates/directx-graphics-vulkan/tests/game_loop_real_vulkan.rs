//! ゲームループプロトタイプ第三歩(2026-08-08)の実機検証。
//! `examples/bouncing_sprite_demo.rs`と同じ「毎フレーム位置を更新→
//! render_sprites_and_read_backで実描画」というループを、実際に複数
//! フレーム分実行し、(1)スプライトが実際にフレームごとに画面上を移動
//! すること、(2)壁で正しく跳ね返ること、を読み戻したピクセル位置から
//! 実証する。

use directx_graphics_vulkan::{render_sprites_and_read_back, Rgba8, SpriteInstance, TextureRgba8};
use directx_shader_translate::spirv_gen::{translate_sprite_pixel_shader, translate_sprite_vertex_shader};

const SPRITE_VS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/sprite_vs.dxbc");
const SPRITE_PS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/sprite_ps.dxbc");

/// NDC x座標を読み戻し画像の列インデックスへ変換する(`render_sprites_and_
/// read_back`のビューポート変換規約: NDC x=-1 -> col=0, x=1 -> col=width-1、
/// 既存の`d3d11_triangle_pixel_position_maps_to_the_expected_ndc_coordinate`
/// で検証済みの変換式と同じ)。
fn ndc_x_to_col(x: f32, width: u32) -> u32 {
    (((x + 1.0) / 2.0) * width as f32).clamp(0.0, width as f32 - 1.0) as u32
}
fn ndc_y_to_row(y: f32, height: u32) -> u32 {
    (((y + 1.0) / 2.0) * height as f32).clamp(0.0, height as f32 - 1.0) as u32
}

#[test]
fn bouncing_sprite_actually_moves_across_frames_and_bounces_off_walls_on_real_vulkan_hardware() {
    let vs = translate_sprite_vertex_shader(SPRITE_VS_DXBC).expect("sprite_vs.dxbc must translate");
    let ps = translate_sprite_pixel_shader(SPRITE_PS_DXBC).expect("sprite_ps.dxbc must translate");

    let ball_color = Rgba8 { r: 255, g: 140, b: 0, a: 255 };
    let texture = TextureRgba8 { width: 1, height: 1, pixels: vec![ball_color] };

    let width = 64u32;
    let height = 64u32;
    let half_size = 0.15f32;

    // 意図的に「右の壁で反射が起きる」ことを1フレーム目で検証できる
    // 初期状態にする: x=0.87がvx=0.10で1ステップ進むと0.97+0.15=1.12>1.0
    // となり反射するはず。
    let mut x = 0.87f32;
    let mut y = 0.0f32;
    let mut vx = 0.10f32;
    let vy = 0.0f32;

    let mut prev_col: Option<u32> = None;
    let mut bounced = false;

    for frame in 0..6 {
        x += vx;
        y += vy;
        if x - half_size < -1.0 || x + half_size > 1.0 {
            vx = -vx;
            x = x.clamp(-1.0 + half_size, 1.0 - half_size);
            bounced = true;
        }

        let dest_ndc = [x - half_size, y - half_size, x + half_size, y + half_size];
        let sprite = SpriteInstance { dest_ndc, uv_rect: [0.0, 0.0, 1.0, 1.0] };
        let pixels = match render_sprites_and_read_back(&vs.spirv_words, &ps.spirv_words, &texture, &[sprite], width, height) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
                return;
            }
        };

        let expected_col = ndc_x_to_col(x, width);
        let expected_row = ndc_y_to_row(y, height);
        let at = pixels[(expected_row * width + expected_col) as usize];
        assert_eq!(
            at, ball_color,
            "frame {frame}: 期待した位置(col={expected_col}, row={expected_row})にボール色が無い(実際={at:?})——\
             ゲームループの位置更新と実際の描画結果がずれている"
        );

        if let Some(prev) = prev_col {
            assert_ne!(
                prev, expected_col,
                "frame {frame}: 前フレームと同じ列にスプライトが留まっている(移動していない)"
            );
        }
        prev_col = Some(expected_col);
    }

    assert!(bounced, "6フレームの間に右の壁での反射が一度も起きなかった(初期状態の想定が外れている)");

    println!(
        "OK: ゲームループ(毎フレームの位置更新+実Vulkan描画)で、スプライトが実際に画面上を移動し、\
         右の壁で正しく跳ね返ることを実機で確認した(6フレーム分、各フレームで期待ピクセル位置の色を検証)"
    );
}
