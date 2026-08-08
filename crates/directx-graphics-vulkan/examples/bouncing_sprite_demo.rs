//! 「まず動かして見る」用のゲームループデモ(2026-08-08、2Dスプライト
//! 描画プロトタイプの第三歩)。`render_triangle.rs`の使いやすさ改善
//! パターンを踏襲し、1コマンドで実際に動くものを見せる。
//!
//! 画面端で跳ね返るスプライトを実際に複数フレーム分レンダリングし、
//! 各フレームをPPM連番ファイルへ保存する(`ffmpeg`等でアニメーション
//! GIF/動画に変換可能)。**正直な開示**: このリポジトリにはウィンドウ/
//! スワップチェーン・実キーボード入力の仕組みが無いため、「入力」は
//! この例では単純な固定初速度による物理シミュレーション(壁での
//! 反射)で代替している——将来、実ウィンドウ+入力デバイスが追加
//! されれば、この`update()`関数の呼び出し元をキー入力ハンドラへ
//! 差し替えるだけで実際のゲームループへ拡張できる設計にしてある。
//!
//! 実行方法:
//! ```bash
//! cargo run -p directx-graphics-vulkan --example bouncing_sprite_demo --release
//! ```

use directx_graphics_vulkan::{render_sprites_and_read_back, Rgba8, SpriteInstance, TextureRgba8};
use directx_shader_translate::spirv_gen::{translate_sprite_pixel_shader, translate_sprite_vertex_shader};

/// スプライトの状態(位置+速度、NDC空間)。壁(NDC[-1,1])での反射のみの
/// 最小限の物理。
struct BallState {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    half_size: f32,
}

impl BallState {
    /// 1フレーム分だけ状態を進める(「入力」に相当する部分——将来、
    /// 実キー入力から速度を設定する形に差し替え可能)。
    fn update(&mut self, dt: f32) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        if self.x - self.half_size < -1.0 || self.x + self.half_size > 1.0 {
            self.vx = -self.vx;
            self.x = self.x.clamp(-1.0 + self.half_size, 1.0 - self.half_size);
        }
        if self.y - self.half_size < -1.0 || self.y + self.half_size > 1.0 {
            self.vy = -self.vy;
            self.y = self.y.clamp(-1.0 + self.half_size, 1.0 - self.half_size);
        }
    }

    fn dest_ndc(&self) -> [f32; 4] {
        [self.x - self.half_size, self.y - self.half_size, self.x + self.half_size, self.y + self.half_size]
    }
}

fn main() {
    let vs = translate_sprite_vertex_shader(include_bytes!("../../directx-shader-translate/shaders/sprite_vs.dxbc"))
        .expect("translate sprite_vs.dxbc");
    let ps = translate_sprite_pixel_shader(include_bytes!("../../directx-shader-translate/shaders/sprite_ps.dxbc"))
        .expect("translate sprite_ps.dxbc");

    // 単色オレンジのボール用テクスチャ(1x1、拡大されてスプライト全体を塗る)。
    let ball_texture = TextureRgba8 { width: 1, height: 1, pixels: vec![Rgba8 { r: 255, g: 140, b: 0, a: 255 }] };

    let width = 64u32;
    let height = 64u32;
    const NUM_FRAMES: usize = 30;

    let mut ball = BallState { x: -0.5, y: -0.3, vx: 0.12, vy: 0.09, half_size: 0.15 };

    for frame in 0..NUM_FRAMES {
        ball.update(1.0);

        let sprite = SpriteInstance { dest_ndc: ball.dest_ndc(), uv_rect: [0.0, 0.0, 1.0, 1.0] };
        let pixels = render_sprites_and_read_back(&vs.spirv_words, &ps.spirv_words, &ball_texture, &[sprite], width, height)
            .unwrap_or_else(|e| {
                eprintln!("実Vulkanデバイスが無いため終了: {e:#}");
                std::process::exit(1);
            });

        let path = format!("bouncing_sprite_frame_{frame:03}.ppm");
        let mut out = format!("P6\n{width} {height}\n255\n").into_bytes();
        for p in &pixels {
            out.extend_from_slice(&[p.r, p.g, p.b]);
        }
        std::fs::write(&path, out).expect("write ppm frame");
    }

    println!(
        "描画成功: 跳ね返るスプライトを{NUM_FRAMES}フレーム分レンダリングし、\
         bouncing_sprite_frame_000.ppm 〜 _{:03}.ppm に保存しました。",
        NUM_FRAMES - 1
    );
    println!(
        "(連番PPMをアニメーションGIF化する例: `ffmpeg -i bouncing_sprite_frame_%03d.ppm bouncing_sprite.gif`)"
    );
}
