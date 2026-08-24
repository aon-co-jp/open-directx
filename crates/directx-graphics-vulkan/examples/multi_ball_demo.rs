//! 複数の動くスプライト+衝突判定のプロトタイプ(2026-08-08、2Dスプライト
//! 描画プロトタイプの続き——`bouncing_sprite_demo.rs`〈ボール1個+壁反射〉
//! を「ボール2個+壁反射+ボール同士の衝突」へ拡張したもの)。
//!
//! 衝突解決は同質量の弾性衝突を簡略化した「衝突時に両者の速度ベクトルを
//! 入れ替える」方式(正面衝突なら物理的に厳密、斜め衝突では近似——
//! 過剰実装を避け、視覚的に自然かつ決定的に検証可能な最小実装とした)。
//!
//! 実行方法:
//! ```bash
//! cargo run -p directx-graphics-vulkan --example multi_ball_demo --release
//! ```

use directx_graphics_vulkan::{render_sprites_and_read_back, Rgba8, SpriteInstance, TextureRgba8};
use directx_shader_translate::spirv_gen::{translate_sprite_pixel_shader, translate_sprite_vertex_shader};

pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub radius: f32,
}

impl Ball {
    pub fn bounce_off_walls(&mut self) {
        if self.x - self.radius < -1.0 || self.x + self.radius > 1.0 {
            self.vx = -self.vx;
            self.x = self.x.clamp(-1.0 + self.radius, 1.0 - self.radius);
        }
        if self.y - self.radius < -1.0 || self.y + self.radius > 1.0 {
            self.vy = -self.vy;
            self.y = self.y.clamp(-1.0 + self.radius, 1.0 - self.radius);
        }
    }

    pub fn dest_ndc(&self) -> [f32; 4] {
        [self.x - self.radius, self.y - self.radius, self.x + self.radius, self.y + self.radius]
    }
}

/// 2球の状態を1フレーム進める(壁反射→移動→球同士の衝突判定の順)。
/// 衝突が実際に発生したかどうかを返す(テスト・デモ双方で使う)。
pub fn update_two_balls(a: &mut Ball, b: &mut Ball) -> bool {
    a.x += a.vx;
    a.y += a.vy;
    b.x += b.vx;
    b.y += b.vy;
    a.bounce_off_walls();
    b.bounce_off_walls();

    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dist_sq = dx * dx + dy * dy;
    let min_dist = a.radius + b.radius;
    if dist_sq < min_dist * min_dist {
        // 等質量弾性衝突の簡略近似: 速度ベクトルを丸ごと入れ替える
        // (正面衝突では物理的に厳密、斜め衝突では近似)。
        std::mem::swap(&mut a.vx, &mut b.vx);
        std::mem::swap(&mut a.vy, &mut b.vy);
        // 押し戻し(めり込み解消、最小限): 重なった分だけ引き離す。
        let dist = dist_sq.sqrt().max(1e-6);
        let overlap = min_dist - dist;
        let nx = dx / dist;
        let ny = dy / dist;
        a.x += nx * overlap * 0.5;
        a.y += ny * overlap * 0.5;
        b.x -= nx * overlap * 0.5;
        b.y -= ny * overlap * 0.5;
        return true;
    }
    false
}

fn main() {
    let vs = translate_sprite_vertex_shader(include_bytes!("../../directx-shader-translate/shaders/sprite_vs.dxbc"))
        .expect("translate sprite_vs.dxbc");
    let ps = translate_sprite_pixel_shader(include_bytes!("../../directx-shader-translate/shaders/sprite_ps.dxbc"))
        .expect("translate sprite_ps.dxbc");

    // 2x1アトラス: 左=赤いボール用、右=水色ボール用(同一テクスチャ引数の
    // 制約〈1回のrender_sprites_and_read_back呼び出しは単一テクスチャ〉
    // を、2026-08-08のアルファブレンドテストと同じ「アトラス+異なる
    // uv_rect」パターンで解決する)。
    let atlas = TextureRgba8 {
        width: 2,
        height: 1,
        pixels: vec![Rgba8 { r: 255, g: 80, b: 80, a: 255 }, Rgba8 { r: 80, g: 220, b: 255, a: 255 }],
    };

    let width = 64u32;
    let height = 64u32;
    const NUM_FRAMES: usize = 40;

    // 正面衝突する初期条件(同じy、水平方向のみの速度)——衝突判定・
    // 速度入れ替えロジックを確実に検証できる決定的な軌道にした。
    let mut a = Ball { x: -0.6, y: 0.0, vx: 0.05, vy: 0.0, radius: 0.12 };
    let mut b = Ball { x: 0.6, y: 0.0, vx: -0.04, vy: 0.0, radius: 0.12 };

    let mut collisions = 0;
    for frame in 0..NUM_FRAMES {
        if update_two_balls(&mut a, &mut b) {
            collisions += 1;
        }

        let sprites = [
            SpriteInstance { dest_ndc: a.dest_ndc(), uv_rect: [0.0, 0.0, 0.5, 1.0] },
            SpriteInstance { dest_ndc: b.dest_ndc(), uv_rect: [0.5, 0.0, 1.0, 1.0] },
        ];
        let pixels = render_sprites_and_read_back(&vs.spirv_words, &ps.spirv_words, &atlas, &sprites, width, height)
            .unwrap_or_else(|e| {
                eprintln!("実Vulkanデバイスが無いため終了: {e:#}");
                std::process::exit(1);
            });

        let path = format!("multi_ball_frame_{frame:03}.ppm");
        let mut out = format!("P6\n{width} {height}\n255\n").into_bytes();
        for p in &pixels {
            out.extend_from_slice(&[p.r, p.g, p.b]);
        }
        std::fs::write(&path, out).expect("write ppm frame");
    }

    println!(
        "描画成功: 2球が衝突する様子を{NUM_FRAMES}フレーム分レンダリングし、\
         multi_ball_frame_000.ppm 〜 _{:03}.ppm に保存しました(衝突検出回数: {collisions})。",
        NUM_FRAMES - 1
    );
}
