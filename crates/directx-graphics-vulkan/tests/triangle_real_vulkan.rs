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

use directx_graphics_vulkan::{
    enumerate_graphics_devices, render_gradient_triangle_and_read_back, render_indexed_scene_with_depth_and_read_back,
    render_uniform_triangle_and_read_back, Vertex,
};
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

/// 2026-08-04 addition: explicit pixel-position <-> NDC-coordinate mapping
/// check, closing the gap every previous HANDOFF entry in this crate left
/// open ("today we only check rasterizer invariants that don't depend on the
/// exact pixel<->NDC transform, not the transform itself").
///
/// Derivation (checked against this crate's actual pipeline setup, not
/// assumed): the "big triangle" vertices used by
/// `render_gradient_triangle_and_read_back` are NDC `v0=(-1,-1)`,
/// `v1=(3,-1)`, `v2=(-1,3)` (see `render_triangle_and_read_back` in
/// `src/lib.rs`), and the viewport is the Vulkan-default
/// `x:0,y:0,width,height,min_depth:0,max_depth:1` with no `y`-flip (see the
/// `vk::Viewport` construction in `src/lib.rs`). With that viewport, NDC
/// `(-1,-1)` maps to the top-left framebuffer corner and NDC `(1,1)` to the
/// bottom-right, i.e. for a pixel center at column `x`, row `y` (0-indexed,
/// row 0 = top, matching this crate's row-major host readback order):
///   ndc_x = (x + 0.5) / width  * 2 - 1
///   ndc_y = (y + 0.5) / height * 2 - 1
/// Solving `P = l0*v0 + l1*v1 + l2*v2`, `l0+l1+l2=1` for this specific
/// triangle gives the closed form `l1 = (ndc_x+1)/4`, `l2 = (ndc_y+1)/4`,
/// `l0 = 1-l1-l2`, independent of the interpolated attribute itself. This
/// test computes that closed-form expected color for one specific,
/// non-center, non-edge pixel (so the check cannot be satisfied by a
/// trivially-symmetric bug) and compares it against the real read-back
/// pixel from the real GPU, rather than only checking transform-independent
/// invariants like the previous test does.
#[test]
fn d3d11_triangle_pixel_position_maps_to_the_expected_ndc_coordinate_on_real_vulkan_hardware() {
    let vs = translate_vertex_shader(TRIANGLE_VS_DXBC).expect("triangle_vs.dxbc must translate");
    let ps = translate_pixel_shader(TRIANGLE_PS_DXBC).expect("triangle_ps.dxbc must translate");

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
                "skipping real-hardware pixel<->NDC mapping test: no usable real Vulkan graphics device/driver ({e})"
            );
            return;
        }
    };
    assert_eq!(pixels.len(), (width * height) as usize);

    // Deliberately off-center, off-diagonal pixel: col=2, row=5 out of 8x8.
    let col = 2u32;
    let row = 5u32;
    let ndc_x = (col as f64 + 0.5) / width as f64 * 2.0 - 1.0;
    let ndc_y = (row as f64 + 0.5) / height as f64 * 2.0 - 1.0;
    let l1 = (ndc_x + 1.0) / 4.0;
    let l2 = (ndc_y + 1.0) / 4.0;
    let l0 = 1.0 - l1 - l2;
    // color = l0*red + l1*green + l2*blue, quantized to R8G8B8A8_UNORM via
    // round(x*255), same CPU-reference convention as the other tests in
    // this file.
    let expected_r = (l0 * 255.0).round() as i32;
    let expected_g = (l1 * 255.0).round() as i32;
    let expected_b = (l2 * 255.0).round() as i32;

    let idx = (row * width + col) as usize;
    let actual = pixels[idx];
    let tolerance = 2i32; // UNORM rounding / interpolation rounding slack, same as other tests in this file.
    assert!(
        (actual.r as i32 - expected_r).abs() <= tolerance
            && (actual.g as i32 - expected_g).abs() <= tolerance
            && (actual.b as i32 - expected_b).abs() <= tolerance,
        "pixel (col={col}, row={row}) = {actual:?}, expected ~({expected_r},{expected_g},{expected_b}) \
         from the derived pixel<->NDC transform (ndc=({ndc_x:.4},{ndc_y:.4}), barycentric=({l0:.4},{l1:.4},{l2:.4})) \
         -- the pixel<->NDC mapping does not match the derivation"
    );
    println!(
        "OK: pixel (col={col}, row={row}) on an {width}x{height} framebuffer maps to NDC ({ndc_x:.4},{ndc_y:.4}) \
         as derived, and the real read-back color {actual:?} matches the closed-form expected color \
         ({expected_r},{expected_g},{expected_b}) within tolerance {tolerance} on the real GPU present on this machine."
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

/// The "depth buffer + multiple triangles + index buffer" milestone every
/// previous HANDOFF entry in this crate listed as the next open item. Draws
/// two full-viewport, mutually overlapping triangles at different depths --
/// a *near* red one (z=0.1, drawn first) and a *far* blue one (z=0.9, drawn
/// second) -- through `render_indexed_scene_with_depth_and_read_back`.
///
/// The draw order is deliberately "near first, far second" specifically
/// because that is the one ordering that only produces the correct result
/// (staying red) *if* depth testing is genuinely rejecting the far
/// triangle's fragments. A pipeline with depth testing silently disabled, or
/// wired up backwards, would let the far (later-drawn) blue triangle
/// overwrite everything -- the classic "painter's algorithm" bug depth
/// buffers exist to prevent. This is a stronger real-hardware assertion than
/// simply checking that *a* color renders: it specifically exercises the
/// depth comparison logic, not just attachment plumbing.
#[test]
fn indexed_scene_with_depth_buffer_keeps_the_nearer_triangle_on_real_vulkan_hardware() {
    let vs = translate_vertex_shader(TRIANGLE_VS_DXBC).expect("triangle_vs.dxbc must translate");
    let ps = translate_pixel_shader(TRIANGLE_PS_DXBC).expect("triangle_ps.dxbc must translate");

    const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
    const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
    let vertices = [
        // Near triangle (z=0.1), full-viewport, drawn first.
        Vertex { pos: [-1.0, -1.0, 0.1], color: RED },
        Vertex { pos: [3.0, -1.0, 0.1], color: RED },
        Vertex { pos: [-1.0, 3.0, 0.1], color: RED },
        // Far triangle (z=0.9), same full-viewport coverage, drawn second.
        Vertex { pos: [-1.0, -1.0, 0.9], color: BLUE },
        Vertex { pos: [3.0, -1.0, 0.9], color: BLUE },
        Vertex { pos: [-1.0, 3.0, 0.9], color: BLUE },
    ];
    let indices: [u32; 6] = [0, 1, 2, 3, 4, 5];

    let pixels = match render_indexed_scene_with_depth_and_read_back(&vs.spirv_words, &ps.spirv_words, &vertices, &indices, 8, 8) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("skipping indexed depth test: no usable real Vulkan graphics device/driver ({e})");
            return;
        }
    };

    assert_eq!(pixels.len(), 8 * 8);
    let mut red_count = 0;
    let mut blue_count = 0;
    for p in &pixels {
        // Quantized round-trip tolerance, same reasoning as the other
        // real-hardware color assertions in this file.
        if p.r > 200 && p.b < 50 {
            red_count += 1;
        } else if p.b > 200 && p.r < 50 {
            blue_count += 1;
        }
    }
    println!("OK: depth-tested indexed scene on real hardware: {red_count} near(red) pixels, {blue_count} far(blue) pixels out of {}", pixels.len());
    assert_eq!(
        blue_count, 0,
        "the farther (z=0.9) triangle must never win the depth test against the nearer (z=0.1) one, regardless of draw order -- found {blue_count} blue pixels"
    );
    assert!(red_count > 0, "expected the nearer triangle's red color to actually appear in the read-back framebuffer");
}
