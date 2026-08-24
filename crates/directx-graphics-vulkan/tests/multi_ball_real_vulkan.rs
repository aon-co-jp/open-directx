//! 複数の動くスプライト+衝突判定の実機検証(2026-08-08)。
//! `examples/multi_ball_demo.rs`と同じシミュレーション
//! (`update_two_balls`)を実際に複数フレーム実行し、(1)衝突前は2球が
//! 独立して実際に移動していること、(2)正面衝突により速度が実際に
//! 入れ替わり、その後の軌道が入れ替わり後の速度と一致すること、を
//! 読み戻したピクセル位置から実証する。

use directx_graphics_vulkan::{render_sprites_and_read_back, Rgba8, SpriteInstance, TextureRgba8};
use directx_shader_translate::spirv_gen::{translate_sprite_pixel_shader, translate_sprite_vertex_shader};

const SPRITE_VS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/sprite_vs.dxbc");
const SPRITE_PS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/sprite_ps.dxbc");

fn ndc_to_col(x: f32, width: u32) -> u32 {
    (((x + 1.0) / 2.0) * width as f32).clamp(0.0, width as f32 - 1.0) as u32
}
fn ndc_to_row(y: f32, height: u32) -> u32 {
    (((y + 1.0) / 2.0) * height as f32).clamp(0.0, height as f32 - 1.0) as u32
}

// `examples/multi_ball_demo.rs`をそのままモジュールとして取り込む
// (`update_two_balls`/`Ball`のみを再利用、この`main`はこのテストからは
// 呼ばれない)。
#[path = "../examples/multi_ball_demo.rs"]
#[allow(dead_code)]
mod demo;

#[test]
fn two_balls_collide_head_on_and_swap_velocities_visibly_on_real_vulkan_hardware() {
    let vs = translate_sprite_vertex_shader(SPRITE_VS_DXBC).expect("sprite_vs.dxbc must translate");
    let ps = translate_sprite_pixel_shader(SPRITE_PS_DXBC).expect("sprite_ps.dxbc must translate");

    let red = Rgba8 { r: 255, g: 80, b: 80, a: 255 };
    let cyan = Rgba8 { r: 80, g: 220, b: 255, a: 255 };
    let atlas = TextureRgba8 { width: 2, height: 1, pixels: vec![red, cyan] };

    let mut a = demo::Ball { x: -0.6, y: 0.0, vx: 0.05, vy: 0.0, radius: 0.12 };
    let mut b = demo::Ball { x: 0.6, y: 0.0, vx: -0.04, vy: 0.0, radius: 0.12 };

    let width = 64u32;
    let height = 64u32;

    let mut collisions = 0;
    let mut prev_a_col: Option<u32> = None;
    let mut prev_b_col: Option<u32> = None;

    for frame in 0..20 {
        let collided = demo::update_two_balls(&mut a, &mut b);
        if collided {
            collisions += 1;
        }

        let sprites = [
            SpriteInstance { dest_ndc: a.dest_ndc(), uv_rect: [0.0, 0.0, 0.5, 1.0] },
            SpriteInstance { dest_ndc: b.dest_ndc(), uv_rect: [0.5, 0.0, 1.0, 1.0] },
        ];
        let pixels = match render_sprites_and_read_back(&vs.spirv_words, &ps.spirv_words, &atlas, &sprites, width, height) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("実Vulkanデバイスが無いためスキップ: {e:#}");
                return;
            }
        };

        let a_col = ndc_to_col(a.x, width);
        let a_row = ndc_to_row(a.y, height);
        let b_col = ndc_to_col(b.x, width);
        let b_row = ndc_to_row(b.y, height);

        assert_eq!(pixels[(a_row * width + a_col) as usize], red, "frame {frame}: 赤ボールの期待位置に赤が無い");
        assert_eq!(pixels[(b_row * width + b_col) as usize], cyan, "frame {frame}: 水色ボールの期待位置に水色が無い");

        if let (Some(pa), Some(pb)) = (prev_a_col, prev_b_col) {
            assert!(
                pa != a_col || pb != b_col,
                "frame {frame}: 前フレームから両球とも同じ位置に留まっている(移動していない)"
            );
        }
        prev_a_col = Some(a_col);
        prev_b_col = Some(b_col);
    }

    assert_eq!(collisions, 1, "20フレームの間に正面衝突がちょうど1回起きるはずの初期条件だった(実測: {collisions}回)");

    // 衝突後、速度が入れ替わっている(赤は元々b由来の負の速度、水色は
    // 元々a由来の正の速度)ことを、シミュレーション状態から直接確認する
    // (レンダリング結果とシミュレーション状態が一貫していることは上の
    // ループ内アサーションで既に検証済みのため、ここでは物理則自体の
    // 検証)。
    assert!(a.vx < 0.0, "衝突後、赤ボールの速度が負(元のbの速度)へ入れ替わっているはず: {}", a.vx);
    assert!(b.vx > 0.0, "衝突後、水色ボールの速度が正(元のaの速度)へ入れ替わっているはず: {}", b.vx);

    println!(
        "OK: 2球の正面衝突+速度入れ替えロジックが、20フレームにわたり実描画結果と一貫していることを実Vulkan経路で確認した \
         (衝突回数={collisions}、衝突後 a.vx={:.3}, b.vx={:.3})",
        a.vx, b.vx
    );
}
