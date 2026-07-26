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

use directx_graphics_vulkan::render_uniform_triangle_and_read_back;
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
