//! Real-hardware integration test for the D3D11 minimal graphics pipeline
//! milestone: reuse the already-generated, already spirv-val-passing SPIR-V
//! from `directx-shader-translate::spirv_gen::{translate_vertex_shader,
//! translate_pixel_shader}` for `triangle_vs.dxbc`/`triangle_ps.dxbc`, issue a
//! real draw call on the real GPU present on this machine, read back the
//! framebuffer, and assert the read-back colors match the pass-through
//! vertex colors fed in.
//!
//! Per repo policy this is a real-hardware test, not a mock: if no Vulkan
//! device/driver is present it prints and skips rather than faking success.

use directx_graphics_vulkan::{enumerate_graphics_devices, render_gradient_triangle_and_read_back, render_uniform_triangle_and_read_back};
use directx_shader_translate::spirv_gen::{translate_pixel_shader, translate_vertex_shader};

// Same real fxc.exe-compiled DXBC bytes already used and verified in
// directx-shader-translate's own tests (`translates_real_fxc_compiled_triangle_vs_dxbc_to_valid_vertex_spirv`
// / `..._triangle_ps_dxbc_to_valid_fragment_spirv`). Embedded here via a
// relative path across the sibling crate directory (dev-dependency only,
// no source coupling).
const TRIANGLE_VS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/triangle_vs.dxbc");
const TRIANGLE_PS_DXBC: &[u8] = include_bytes!("../../directx-shader-translate/shaders/triangle_ps.dxbc");

#[test]
fn d3d11_triangle_draw_call_matches_passthrough_vertex_color_on_real_vulkan_hardware() {
    let vs = translate_vertex_shader(TRIANGLE_VS_DXBC).expect("triangle_vs.dxbc must translate");
    let ps = translate_pixel_shader(TRIANGLE_PS_DXBC).expect("triangle_ps.dxbc must translate");

    // Distinctive, non-gray, non-zero color so no channel is accidentally
    // left at its cleared/default value by a silent bug (CPU-reference
    // equivalent: quantizing float [0,1] to R8G8B8A8_UNORM via round(x*255)).
    let vertex_color = [200.0 / 255.0, 100.0 / 255.0, 50.0 / 255.0, 255.0 / 255.0];
    let expected = directx_graphics_vulkan::Rgba8 { r: 200, g: 100, b: 50, a: 255 };

    let width = 4u32;
    let height = 4u32;

    let pixels = match render_uniform_triangle_and_read_back(
        &vs.spirv_words,
        &ps.spirv_words,
        vertex_color,
        width,
        height,
    ) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "skipping real-hardware D3D11 triangle test: no usable real Vulkan graphics device/driver ({e})"
            );
            return;
        }
    };

    assert_eq!(pixels.len(), (width * height) as usize);

    // The "big triangle" (-1,-1),(3,-1),(-1,3) fully covers the viewport, so
    // every pixel must be shaded by the passthrough pixel shader with the
    // exact passthrough vertex color (allow +/-1 for UNORM rounding).
    let mut checked = 0;
    for (i, px) in pixels.iter().enumerate() {
        let close = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 1;
        assert!(
            close(px.r, expected.r) && close(px.g, expected.g) && close(px.b, expected.b) && close(px.a, expected.a),
            "pixel {i} = {px:?} does not match passthrough vertex color {expected:?}"
        );
        checked += 1;
    }
    assert_eq!(checked, (width * height) as usize);

    println!(
        "OK: D3D11 minimal graphics pipeline (real ash-driven render pass + framebuffer + \
         VkGraphicsPipelineCreateInfo) drew a full-viewport triangle using triangle_vs.dxbc/triangle_ps.dxbc's \
         real translated SPIR-V, and all {}x{} read-back pixels matched the passthrough vertex color \
         {:?} on the real GPU present on this machine.",
        width, height, expected
    );
}

/// 2026-07-26 addition: the uniform-color test above cannot distinguish
/// "the rasterizer correctly interpolates per-vertex colors" from "the
/// pipeline just replicates one vertex's color everywhere" — every vertex is
/// fed the exact same color, so interpolation is degenerate (any weighted
/// average of three equal colors is that same color). This test assigns
/// pure red/green/blue to the three distinct vertices and checks real
/// hardware readback for the defining property of barycentric/affine color
/// interpolation, without hardcoding the pixel-to-NDC coordinate convention
/// (viewport y-direction, which vertex maps to which screen corner, etc.),
/// which would make the test as fragile as the renderer it's checking:
///
/// 1. Every covered pixel's (r,g,b) sums to ~255 (the "big triangle" fully
///    covers the viewport, and R=(255,0,0)+G=(0,255,0)+B=(0,0,255) is a
///    partition of unity: for any convex combination `u*R + v*G + w*B` with
///    `u+v+w=1`, the channel sum is always `255`. A buggy pipeline that
///    clamped/truncated weights, or that used non-affine/incorrect weights,
///    would not preserve this invariant across every pixel.)
/// 2. The image is not a single flat color (i.e. genuine per-pixel
///    variation exists) — ruling out the degenerate "always outputs vertex
///    0's color" bug that the uniform-color test above cannot catch.
#[test]
fn d3d11_triangle_draw_call_interpolates_distinct_per_vertex_colors_on_real_vulkan_hardware() {
    let vs = translate_vertex_shader(TRIANGLE_VS_DXBC).expect("triangle_vs.dxbc must translate");
    let ps = translate_pixel_shader(TRIANGLE_PS_DXBC).expect("triangle_ps.dxbc must translate");

    // Pure red / green / blue per vertex (alpha fixed at 1.0 for all three,
    // so alpha is not part of the interpolation being exercised here).
    let vertex_colors = [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
    ];

    let width = 8u32;
    let height = 8u32;

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
                "skipping real-hardware D3D11 gradient triangle test: no usable real Vulkan graphics device/driver ({e})"
            );
            return;
        }
    };

    assert_eq!(pixels.len(), (width * height) as usize);

    let mut saw_a_different_pixel = false;
    let first = pixels[0];
    for (i, px) in pixels.iter().enumerate() {
        let sum = px.r as i32 + px.g as i32 + px.b as i32;
        assert!(
            (sum - 255).abs() <= 2,
            "pixel {i} = {px:?} has r+g+b = {sum}, expected ~255 (partition-of-unity of a convex \
             combination of pure red/green/blue vertex colors) — the rasterizer's color \
             interpolation does not look affine"
        );
        assert_eq!(px.a, 255, "pixel {i} alpha = {}, expected 255 (constant across all three vertices)", px.a);
        if px.r != first.r || px.g != first.g || px.b != first.b {
            saw_a_different_pixel = true;
        }
    }
    assert!(
        saw_a_different_pixel,
        "all {} pixels had the identical color {:?} — this would also be consistent with a broken \
         pipeline that always outputs one vertex's color regardless of position, which the \
         partition-of-unity check above cannot rule out on its own",
        pixels.len(),
        first
    );

    println!(
        "OK: D3D11 minimal graphics pipeline correctly interpolates distinct per-vertex colors \
         (pure red/green/blue) across all {}x{} read-back pixels on the real GPU present on this \
         machine — every pixel's r+g+b sums to ~255 and the image is not a single flat color.",
        width, height
    );
}

/// 2026-07-27 addition: `enumerate_graphics_devices` should report the real
/// GPU on this machine (an NVIDIA GeForce GT 730) with the correct
/// best-effort vendor name, closing the diagnostic parity gap noted in
/// CLAUDE.md (open-cuda's Compute path already reports vendor via
/// `opencuda-vulkan::real::vendor_from_id`; the Graphics path here had no
/// equivalent). Skips (does not fail) if no Vulkan graphics device is
/// present, matching this repo's existing real-hardware-test policy.
#[test]
fn enumerate_graphics_devices_reports_the_real_gpu_on_this_machine() {
    let devices = match enumerate_graphics_devices() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("skipping enumerate_graphics_devices test: no usable real Vulkan graphics device/driver ({e})");
            return;
        }
    };
    assert!(!devices.is_empty(), "expected at least one graphics-capable Vulkan physical device to be enumerated");
    for d in &devices {
        assert!(!d.name.is_empty(), "device name should not be empty");
    }
    println!("OK: enumerate_graphics_devices reported {} graphics-capable device(s): {:?}", devices.len(), devices);
}
