//! Minimal real Vulkan graphics pipeline for the D3D11 "minimal graphics
//! pipeline" milestone: render pass + framebuffer + `VkGraphicsPipelineCreateInfo`,
//! reusing the already-generated, already `spirv-val`-passing SPIR-V produced by
//! `directx-shader-translate::spirv_gen::{translate_vertex_shader, translate_pixel_shader}`.
//!
//! This crate does **not** wrap `opencuda-vulkan` (that crate only exposes
//! compute-dispatch Vulkan, no graphics pipeline code — verified by source
//! audit, see `open-directx/CLAUDE.md` HANDOFF 2026-07-26). It depends on
//! `ash` directly.
//!
//! Scope (narrow but real, matching the rest of this repo's honesty policy):
//! this is a single hardcoded offscreen render of one full-viewport triangle
//! (passthrough VS + passthrough PS), rendered to a small device-local color
//! image, read back through a host-visible staging buffer. Two entry points
//! share the same pipeline setup: [`render_uniform_triangle_and_read_back`]
//! assigns every vertex the same color (verifying the passthrough shaders
//! reproduce the input color unchanged, with no ambiguity from
//! interpolation), and [`render_gradient_triangle_and_read_back`] (added
//! 2026-07-26) assigns a distinct color per vertex, so the tests can verify
//! the rasterizer's actual barycentric color interpolation rather than only
//! the degenerate all-equal case. It is not a general-purpose renderer: no
//! swapchain, no depth buffer, no textures, no per-draw parameterization
//! beyond what the tests below exercise.

use std::ffi::CString;

use ash::{vk, Entry};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphicsError {
    #[error("failed to load the Vulkan loader: {0}")]
    LoaderLoad(String),
    #[error("vkCreateInstance failed: {0}")]
    CreateInstance(String),
    #[error("no Vulkan physical device with a graphics queue family was found")]
    NoGraphicsDevice,
    #[error("Vulkan call {0} failed: {1}")]
    Vk(&'static str, vk::Result),
}

type Result<T> = std::result::Result<T, GraphicsError>;

/// RGBA8 color, matching what a `COLOR` vertex attribute (float4, 0..1 range)
/// would produce once quantized to an `R8G8B8A8_UNORM` framebuffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Diagnostic info about a Vulkan physical device usable for graphics
/// (i.e. it has at least one queue family with `GRAPHICS` support — the
/// same selection criterion [`render_uniform_triangle_and_read_back`] uses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsDeviceInfo {
    pub name: String,
    pub vendor_id: u32,
    /// Human-readable vendor name, best-effort from the PCI vendor ID
    /// (2026-07-27 addition, closing a parity gap noted in this repo's
    /// CLAUDE.md: `open-cuda`'s `opencuda-vulkan::real::vendor_from_id`
    /// already reports this for the Compute path, but the independent
    /// Graphics path here had no equivalent diagnostic. This is a
    /// deliberately small, standalone duplicate of that same PCI vendor ID
    /// table — not a dependency on `opencuda-vulkan`, consistent with this
    /// crate's existing documented decision to keep the Graphics and
    /// Compute Vulkan contexts fully independent).
    pub vendor: &'static str,
}

fn vendor_name_from_id(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x10DE => "NVIDIA",
        0x1002 | 0x1022 => "AMD",
        0x8086 => "Intel",
        0x5143 => "Qualcomm",
        0x13B5 => "ARM",
        0x1010 => "Imagination PowerVR",
        _ => "Unknown",
    }
}

/// Enumerate every Vulkan physical device with a graphics-capable queue
/// family (the same criterion used to pick the device that
/// [`render_uniform_triangle_and_read_back`]/[`render_gradient_triangle_and_read_back`]
/// actually renders on), returning each device's name and best-effort
/// vendor. Diagnostics only — does not create a logical device, does not
/// render anything, and has no effect on the render functions above.
pub fn enumerate_graphics_devices() -> Result<Vec<GraphicsDeviceInfo>> {
    let entry = unsafe { Entry::load() }.map_err(|e| GraphicsError::LoaderLoad(e.to_string()))?;

    let app_name = CString::new("open-directx").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&app_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_1);
    let instance_info = vk::InstanceCreateInfo::builder().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&instance_info, None) }.map_err(|e| GraphicsError::CreateInstance(e.to_string()))?;
    let guard = InstanceGuard { instance: &instance };

    let physical_devices = unsafe { instance.enumerate_physical_devices() }.map_err(|e| GraphicsError::Vk("vkEnumeratePhysicalDevices", e))?;

    let mut out = Vec::new();
    for &pd in &physical_devices {
        let families = unsafe { instance.get_physical_device_queue_family_properties(pd) };
        let has_graphics = families.iter().any(|f| f.queue_flags.contains(vk::QueueFlags::GRAPHICS));
        if !has_graphics {
            continue;
        }
        let props = unsafe { instance.get_physical_device_properties(pd) };
        let name = unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) }.to_string_lossy().into_owned();
        out.push(GraphicsDeviceInfo { name, vendor_id: props.vendor_id, vendor: vendor_name_from_id(props.vendor_id) });
    }

    drop(guard);
    Ok(out)
}

/// Render a single full-viewport triangle (NDC vertices `(-1,-1)`, `(3,-1)`,
/// `(-1,3)` — the standard "big triangle" trick that fully covers the
/// viewport with one triangle instead of two) using the given passthrough
/// vertex/pixel shader SPIR-V, with every vertex assigned `vertex_color`
/// (so interpolation across the triangle cannot introduce ambiguity: the
/// passthrough pixel shader must output exactly `vertex_color` at every
/// covered pixel). Returns the `width * height` RGBA8 pixels read back from
/// the rendered image, in row-major order starting at the top-left texel.
pub fn render_uniform_triangle_and_read_back(
    vs_spirv: &[u32],
    ps_spirv: &[u32],
    vertex_color: [f32; 4],
    width: u32,
    height: u32,
) -> Result<Vec<Rgba8>> {
    render_triangle_and_read_back(vs_spirv, ps_spirv, [vertex_color; 3], width, height)
}

/// Render the same full-viewport "big triangle" as
/// [`render_uniform_triangle_and_read_back`], but with a distinct color per
/// vertex (`vertex_colors[0]` for NDC `(-1,-1)`, `[1]` for `(3,-1)`, `[2]` for
/// `(-1,3)`), exercising the rasterizer's actual barycentric color
/// interpolation instead of the degenerate all-vertices-equal case. Returns
/// the same row-major RGBA8 readback as the uniform-color variant.
pub fn render_gradient_triangle_and_read_back(
    vs_spirv: &[u32],
    ps_spirv: &[u32],
    vertex_colors: [[f32; 4]; 3],
    width: u32,
    height: u32,
) -> Result<Vec<Rgba8>> {
    render_triangle_and_read_back(vs_spirv, ps_spirv, vertex_colors, width, height)
}

fn render_triangle_and_read_back(
    vs_spirv: &[u32],
    ps_spirv: &[u32],
    vertex_colors: [[f32; 4]; 3],
    width: u32,
    height: u32,
) -> Result<Vec<Rgba8>> {
    let entry =
        unsafe { Entry::load() }.map_err(|e| GraphicsError::LoaderLoad(e.to_string()))?;

    let app_name = CString::new("open-directx").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&app_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_1);
    let instance_info = vk::InstanceCreateInfo::builder().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&instance_info, None) }
        .map_err(|e| GraphicsError::CreateInstance(e.to_string()))?;

    // RAII-ish guard: destroy the instance on any early return.
    let guard = InstanceGuard { instance: &instance };

    let physical_devices = unsafe { instance.enumerate_physical_devices() }
        .map_err(|e| GraphicsError::Vk("vkEnumeratePhysicalDevices", e))?;

    let mut selected: Option<(vk::PhysicalDevice, u32)> = None;
    for &pd in &physical_devices {
        let families = unsafe { instance.get_physical_device_queue_family_properties(pd) };
        if let Some((idx, _)) = families
            .iter()
            .enumerate()
            .find(|(_, f)| f.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        {
            selected = Some((pd, idx as u32));
            break;
        }
    }
    let (physical_device, queue_family_index) = selected.ok_or(GraphicsError::NoGraphicsDevice)?;

    let priorities = [1.0f32];
    let queue_info = [vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities)
        .build()];
    let device_info = vk::DeviceCreateInfo::builder().queue_create_infos(&queue_info);
    let device = unsafe { instance.create_device(physical_device, &device_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateDevice", e))?;
    let dguard = DeviceGuard { device: &device };

    let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
    let memory_properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };

    const FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

    // --- Color attachment image (device-local, render target + copy source) ---
    let image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(FORMAT)
        .extent(vk::Extent3D { width, height, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image = unsafe { device.create_image(&image_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateImage", e))?;
    let image_mem_req = unsafe { device.get_image_memory_requirements(image) };
    let image_mem_type = find_memory_type(
        &memory_properties,
        image_mem_req.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
    .ok_or(GraphicsError::NoGraphicsDevice)?;
    let image_alloc = vk::MemoryAllocateInfo::builder()
        .allocation_size(image_mem_req.size)
        .memory_type_index(image_mem_type);
    let image_memory = unsafe { device.allocate_memory(&image_alloc, None) }
        .map_err(|e| GraphicsError::Vk("vkAllocateMemory(image)", e))?;
    unsafe { device.bind_image_memory(image, image_memory, 0) }
        .map_err(|e| GraphicsError::Vk("vkBindImageMemory", e))?;

    let view_info = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(FORMAT)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    let image_view = unsafe { device.create_image_view(&view_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateImageView", e))?;

    // --- Render pass: one color attachment, final layout TRANSFER_SRC_OPTIMAL
    //     so the render pass itself performs the layout transition we need
    //     for the subsequent vkCmdCopyImageToBuffer (no manual barrier). ---
    let attachment = vk::AttachmentDescription::builder()
        .format(FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .build();
    let color_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };
    let color_refs = [color_ref];
    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_refs)
        .build();
    let attachments = [attachment];
    let subpasses = [subpass];
    // Explicit external -> subpass0 dependency so the CLEAR/write in the
    // color attachment is correctly ordered relative to nothing else running
    // concurrently on this queue (single submit, but keeps validation happy
    // and is the documented-correct pattern).
    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .build();
    let dependencies = [dependency];
    let render_pass_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);
    let render_pass = unsafe { device.create_render_pass(&render_pass_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateRenderPass", e))?;

    let attachments_fb = [image_view];
    let framebuffer_info = vk::FramebufferCreateInfo::builder()
        .render_pass(render_pass)
        .attachments(&attachments_fb)
        .width(width)
        .height(height)
        .layers(1);
    let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateFramebuffer", e))?;

    // --- Vertex buffer: 3 vertices, POSITION(vec3) + COLOR(vec4) interleaved,
    //     matching triangle_vs's SPIR-V input layout (Location 0 = POSITION,
    //     Location 1 = COLOR). Full-viewport "big triangle". ---
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Vertex {
        pos: [f32; 3],
        color: [f32; 4],
    }
    let vertices = [
        Vertex { pos: [-1.0, -1.0, 0.0], color: vertex_colors[0] },
        Vertex { pos: [3.0, -1.0, 0.0], color: vertex_colors[1] },
        Vertex { pos: [-1.0, 3.0, 0.0], color: vertex_colors[2] },
    ];
    let vbuf_size = std::mem::size_of_val(&vertices) as vk::DeviceSize;
    let (vertex_buffer, vertex_buffer_memory) = create_host_visible_buffer(
        &device,
        &memory_properties,
        vbuf_size,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )?;
    unsafe {
        let ptr = device
            .map_memory(vertex_buffer_memory, 0, vbuf_size, vk::MemoryMapFlags::empty())
            .map_err(|e| GraphicsError::Vk("vkMapMemory(vertex)", e))?;
        std::ptr::copy_nonoverlapping(vertices.as_ptr() as *const u8, ptr as *mut u8, vbuf_size as usize);
        device.unmap_memory(vertex_buffer_memory);
    }

    // --- Shader modules from the already spirv-val-passing translated SPIR-V. ---
    let vs_module_info = vk::ShaderModuleCreateInfo::builder().code(vs_spirv);
    let vs_module = unsafe { device.create_shader_module(&vs_module_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateShaderModule(vs)", e))?;
    let ps_module_info = vk::ShaderModuleCreateInfo::builder().code(ps_spirv);
    let ps_module = unsafe { device.create_shader_module(&ps_module_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateShaderModule(ps)", e))?;

    let entry_name = CString::new("main").unwrap();
    let stages = [
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vs_module)
            .name(&entry_name)
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(ps_module)
            .name(&entry_name)
            .build(),
    ];

    let binding_desc = vk::VertexInputBindingDescription {
        binding: 0,
        stride: std::mem::size_of::<Vertex>() as u32,
        input_rate: vk::VertexInputRate::VERTEX,
    };
    let attr_descs = [
        vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32B32_SFLOAT,
            offset: 0,
        },
        vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: std::mem::size_of::<[f32; 3]>() as u32,
        },
    ];
    let binding_descs = [binding_desc];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&binding_descs)
        .vertex_attribute_descriptions(&attr_descs);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: width as f32,
        height: height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    };
    let scissor = vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: vk::Extent2D { width, height },
    };
    let viewports = [viewport];
    let scissors = [scissor];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewports)
        .scissors(&scissors);

    let rasterization = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false)
        .line_width(1.0);

    let multisample = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .sample_shading_enable(false);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
        blend_enable: vk::FALSE,
        color_write_mask: vk::ColorComponentFlags::RGBA,
        ..Default::default()
    };
    let color_blend_attachments = [color_blend_attachment];
    let color_blend = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);

    let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder();
    let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreatePipelineLayout", e))?;

    let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisample)
        .color_blend_state(&color_blend)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .build();
    let pipelines = unsafe {
        device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
    }
    .map_err(|(_, e)| GraphicsError::Vk("vkCreateGraphicsPipelines", e))?;
    let pipeline = pipelines[0];

    // --- Readback (host-visible) buffer ---
    let readback_size = (width * height * 4) as vk::DeviceSize;
    let (readback_buffer, readback_memory) = create_host_visible_buffer(
        &device,
        &memory_properties,
        readback_size,
        vk::BufferUsageFlags::TRANSFER_DST,
    )?;

    // --- Command pool/buffer, record + submit ---
    let pool_info = vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family_index);
    let command_pool = unsafe { device.create_command_pool(&pool_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateCommandPool", e))?;
    let cmd_alloc_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let cmd_buffers = unsafe { device.allocate_command_buffers(&cmd_alloc_info) }
        .map_err(|e| GraphicsError::Vk("vkAllocateCommandBuffers", e))?;
    let cmd = cmd_buffers[0];

    let begin_info = vk::CommandBufferBeginInfo::builder();
    unsafe { device.begin_command_buffer(cmd, &begin_info) }
        .map_err(|e| GraphicsError::Vk("vkBeginCommandBuffer", e))?;

    let clear_value = vk::ClearValue {
        color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] },
    };
    let clear_values = [clear_value];
    let render_pass_begin = vk::RenderPassBeginInfo::builder()
        .render_pass(render_pass)
        .framebuffer(framebuffer)
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        })
        .clear_values(&clear_values);
    unsafe {
        device.cmd_begin_render_pass(cmd, &render_pass_begin, vk::SubpassContents::INLINE);
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
        device.cmd_bind_vertex_buffers(cmd, 0, &[vertex_buffer], &[0]);
        device.cmd_draw(cmd, 3, 1, 0, 0);
        device.cmd_end_render_pass(cmd);

        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D { width, height, depth: 1 },
        };
        // The render pass's final_layout already transitioned the image to
        // TRANSFER_SRC_OPTIMAL, so no extra pipeline barrier is required here.
        device.cmd_copy_image_to_buffer(
            cmd,
            image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            readback_buffer,
            &[region],
        );
    }
    unsafe { device.end_command_buffer(cmd) }.map_err(|e| GraphicsError::Vk("vkEndCommandBuffer", e))?;

    let fence_info = vk::FenceCreateInfo::builder();
    let fence = unsafe { device.create_fence(&fence_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateFence", e))?;
    let cmds = [cmd];
    let submit_info = vk::SubmitInfo::builder().command_buffers(&cmds).build();
    unsafe { device.queue_submit(queue, &[submit_info], fence) }
        .map_err(|e| GraphicsError::Vk("vkQueueSubmit", e))?;
    unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }
        .map_err(|e| GraphicsError::Vk("vkWaitForFences", e))?;

    let pixels = unsafe {
        let ptr = device
            .map_memory(readback_memory, 0, readback_size, vk::MemoryMapFlags::empty())
            .map_err(|e| GraphicsError::Vk("vkMapMemory(readback)", e))?;
        let slice = std::slice::from_raw_parts(ptr as *const u8, readback_size as usize);
        let mut out = Vec::with_capacity((width * height) as usize);
        for chunk in slice.chunks_exact(4) {
            out.push(Rgba8 { r: chunk[0], g: chunk[1], b: chunk[2], a: chunk[3] });
        }
        device.unmap_memory(readback_memory);
        out
    };

    // --- Cleanup (best-effort; process exit also reclaims everything, but
    //     be tidy since this is a library function that may be called more
    //     than once in a test binary). ---
    unsafe {
        device.destroy_fence(fence, None);
        device.destroy_command_pool(command_pool, None);
        device.destroy_pipeline(pipeline, None);
        device.destroy_pipeline_layout(pipeline_layout, None);
        device.destroy_shader_module(vs_module, None);
        device.destroy_shader_module(ps_module, None);
        device.destroy_framebuffer(framebuffer, None);
        device.destroy_render_pass(render_pass, None);
        device.destroy_image_view(image_view, None);
        device.destroy_image(image, None);
        device.free_memory(image_memory, None);
        device.destroy_buffer(vertex_buffer, None);
        device.free_memory(vertex_buffer_memory, None);
        device.destroy_buffer(readback_buffer, None);
        device.free_memory(readback_memory, None);
    }
    drop(dguard);
    drop(guard);

    Ok(pixels)
}

/// A single vertex for [`render_indexed_scene_with_depth_and_read_back`]:
/// NDC-space position (z used for depth testing, not just discarded as the
/// single-triangle helpers above effectively do) plus an RGBA color that the
/// existing pass-through pixel shader (`triangle_ps.dxbc`) forwards
/// unmodified, exactly as in [`render_gradient_triangle_and_read_back`].
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

/// The "more substantial D3D11 draw command" milestone that every previous
/// HANDOFF entry in this crate listed as the next increment: a depth buffer,
/// an arbitrary vertex/index list (so more than one triangle can be drawn in
/// a single pass), and `vkCmdDrawIndexed` instead of the single hardcoded
/// full-viewport "big triangle" the other functions in this file render.
///
/// This is a separate, self-contained function rather than a refactor of
/// [`render_triangle_and_read_back`] on purpose: that function's 3-vertex,
/// no-index, no-depth pipeline is exercised by real-hardware tests that
/// already pass on the one GPU available in this environment (NVIDIA GT 730),
/// and reshaping it to grow an optional depth attachment + index buffer would
/// risk regressing those without a second GPU to cross-check against. The
/// duplication here is the same trade-off already made by
/// `render_gradient_triangle_and_read_back` (a thin wrapper) vs. this
/// (a genuinely different pipeline shape) -- intentional, not an oversight.
///
/// `indices` must be non-empty and every value must be `< vertices.len()`.
/// Depth testing uses `vk::CompareOp::LESS` with `min_depth=0.0`/
/// `max_depth=1.0`, matching D3D11's default (smaller = nearer) convention.
pub fn render_indexed_scene_with_depth_and_read_back(
    vs_spirv: &[u32],
    ps_spirv: &[u32],
    vertices: &[Vertex],
    indices: &[u32],
    width: u32,
    height: u32,
) -> Result<Vec<Rgba8>> {
    if indices.is_empty() {
        return Err(GraphicsError::NoGraphicsDevice);
    }

    let entry = unsafe { Entry::load() }.map_err(|e| GraphicsError::LoaderLoad(e.to_string()))?;

    let app_name = CString::new("open-directx").unwrap();
    let app_info = vk::ApplicationInfo::builder()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&app_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_1);
    let instance_info = vk::InstanceCreateInfo::builder().application_info(&app_info);
    let instance = unsafe { entry.create_instance(&instance_info, None) }
        .map_err(|e| GraphicsError::CreateInstance(e.to_string()))?;
    let guard = InstanceGuard { instance: &instance };

    let physical_devices = unsafe { instance.enumerate_physical_devices() }
        .map_err(|e| GraphicsError::Vk("vkEnumeratePhysicalDevices", e))?;
    let mut selected: Option<(vk::PhysicalDevice, u32)> = None;
    for &pd in &physical_devices {
        let families = unsafe { instance.get_physical_device_queue_family_properties(pd) };
        if let Some((idx, _)) = families
            .iter()
            .enumerate()
            .find(|(_, f)| f.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        {
            selected = Some((pd, idx as u32));
            break;
        }
    }
    let (physical_device, queue_family_index) = selected.ok_or(GraphicsError::NoGraphicsDevice)?;

    let priorities = [1.0f32];
    let queue_info = [vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities)
        .build()];
    let device_info = vk::DeviceCreateInfo::builder().queue_create_infos(&queue_info);
    let device = unsafe { instance.create_device(physical_device, &device_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateDevice", e))?;
    let dguard = DeviceGuard { device: &device };

    let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
    let memory_properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };

    const FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
    const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

    // --- Color attachment (identical to render_triangle_and_read_back). ---
    let image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(FORMAT)
        .extent(vk::Extent3D { width, height, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let image = unsafe { device.create_image(&image_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateImage(color)", e))?;
    let image_mem_req = unsafe { device.get_image_memory_requirements(image) };
    let image_mem_type = find_memory_type(&memory_properties, image_mem_req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
        .ok_or(GraphicsError::NoGraphicsDevice)?;
    let image_alloc = vk::MemoryAllocateInfo::builder().allocation_size(image_mem_req.size).memory_type_index(image_mem_type);
    let image_memory = unsafe { device.allocate_memory(&image_alloc, None) }
        .map_err(|e| GraphicsError::Vk("vkAllocateMemory(color)", e))?;
    unsafe { device.bind_image_memory(image, image_memory, 0) }.map_err(|e| GraphicsError::Vk("vkBindImageMemory(color)", e))?;
    let view_info = vk::ImageViewCreateInfo::builder().image(image).view_type(vk::ImageViewType::TYPE_2D).format(FORMAT).subresource_range(
        vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 },
    );
    let image_view = unsafe { device.create_image_view(&view_info, None) }.map_err(|e| GraphicsError::Vk("vkCreateImageView(color)", e))?;

    // --- Depth attachment (device-local, D32_SFLOAT, not read back -- only
    //     used to gate which fragments actually reach the color buffer). ---
    let depth_image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(DEPTH_FORMAT)
        .extent(vk::Extent3D { width, height, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);
    let depth_image = unsafe { device.create_image(&depth_image_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateImage(depth)", e))?;
    let depth_mem_req = unsafe { device.get_image_memory_requirements(depth_image) };
    let depth_mem_type = find_memory_type(&memory_properties, depth_mem_req.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL)
        .ok_or(GraphicsError::NoGraphicsDevice)?;
    let depth_alloc = vk::MemoryAllocateInfo::builder().allocation_size(depth_mem_req.size).memory_type_index(depth_mem_type);
    let depth_memory = unsafe { device.allocate_memory(&depth_alloc, None) }
        .map_err(|e| GraphicsError::Vk("vkAllocateMemory(depth)", e))?;
    unsafe { device.bind_image_memory(depth_image, depth_memory, 0) }.map_err(|e| GraphicsError::Vk("vkBindImageMemory(depth)", e))?;
    let depth_view_info = vk::ImageViewCreateInfo::builder().image(depth_image).view_type(vk::ImageViewType::TYPE_2D).format(DEPTH_FORMAT)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::DEPTH, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    let depth_view = unsafe { device.create_image_view(&depth_view_info, None) }.map_err(|e| GraphicsError::Vk("vkCreateImageView(depth)", e))?;

    // --- Render pass: color attachment 0 (as before) + depth attachment 1. ---
    let color_attachment = vk::AttachmentDescription::builder()
        .format(FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .build();
    let depth_attachment = vk::AttachmentDescription::builder()
        .format(DEPTH_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();
    let color_ref = vk::AttachmentReference { attachment: 0, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL };
    let depth_ref = vk::AttachmentReference { attachment: 1, layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL };
    let color_refs = [color_ref];
    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_refs)
        .depth_stencil_attachment(&depth_ref)
        .build();
    let attachments = [color_attachment, depth_attachment];
    let subpasses = [subpass];
    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
        .build();
    let dependencies = [dependency];
    let render_pass_info = vk::RenderPassCreateInfo::builder().attachments(&attachments).subpasses(&subpasses).dependencies(&dependencies);
    let render_pass = unsafe { device.create_render_pass(&render_pass_info, None) }.map_err(|e| GraphicsError::Vk("vkCreateRenderPass", e))?;

    let attachments_fb = [image_view, depth_view];
    let framebuffer_info = vk::FramebufferCreateInfo::builder().render_pass(render_pass).attachments(&attachments_fb).width(width).height(height).layers(1);
    let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None) }.map_err(|e| GraphicsError::Vk("vkCreateFramebuffer", e))?;

    // --- Vertex + index buffers (arbitrary caller-supplied geometry, unlike
    //     the fixed 3-vertex "big triangle" elsewhere in this file). ---
    let vbuf_size = std::mem::size_of_val(vertices) as vk::DeviceSize;
    let (vertex_buffer, vertex_buffer_memory) =
        create_host_visible_buffer(&device, &memory_properties, vbuf_size, vk::BufferUsageFlags::VERTEX_BUFFER)?;
    unsafe {
        let ptr = device.map_memory(vertex_buffer_memory, 0, vbuf_size, vk::MemoryMapFlags::empty()).map_err(|e| GraphicsError::Vk("vkMapMemory(vertex)", e))?;
        std::ptr::copy_nonoverlapping(vertices.as_ptr() as *const u8, ptr as *mut u8, vbuf_size as usize);
        device.unmap_memory(vertex_buffer_memory);
    }

    let ibuf_size = std::mem::size_of_val(indices) as vk::DeviceSize;
    let (index_buffer, index_buffer_memory) =
        create_host_visible_buffer(&device, &memory_properties, ibuf_size, vk::BufferUsageFlags::INDEX_BUFFER)?;
    unsafe {
        let ptr = device.map_memory(index_buffer_memory, 0, ibuf_size, vk::MemoryMapFlags::empty()).map_err(|e| GraphicsError::Vk("vkMapMemory(index)", e))?;
        std::ptr::copy_nonoverlapping(indices.as_ptr() as *const u8, ptr as *mut u8, ibuf_size as usize);
        device.unmap_memory(index_buffer_memory);
    }

    let vs_module_info = vk::ShaderModuleCreateInfo::builder().code(vs_spirv);
    let vs_module = unsafe { device.create_shader_module(&vs_module_info, None) }.map_err(|e| GraphicsError::Vk("vkCreateShaderModule(vs)", e))?;
    let ps_module_info = vk::ShaderModuleCreateInfo::builder().code(ps_spirv);
    let ps_module = unsafe { device.create_shader_module(&ps_module_info, None) }.map_err(|e| GraphicsError::Vk("vkCreateShaderModule(ps)", e))?;

    let entry_name = CString::new("main").unwrap();
    let stages = [
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::VERTEX).module(vs_module).name(&entry_name).build(),
        vk::PipelineShaderStageCreateInfo::builder().stage(vk::ShaderStageFlags::FRAGMENT).module(ps_module).name(&entry_name).build(),
    ];

    let binding_desc = vk::VertexInputBindingDescription { binding: 0, stride: std::mem::size_of::<Vertex>() as u32, input_rate: vk::VertexInputRate::VERTEX };
    let attr_descs = [
        vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32B32_SFLOAT, offset: 0 },
        vk::VertexInputAttributeDescription { location: 1, binding: 0, format: vk::Format::R32G32B32A32_SFLOAT, offset: std::mem::size_of::<[f32; 3]>() as u32 },
    ];
    let binding_descs = [binding_desc];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder().vertex_binding_descriptions(&binding_descs).vertex_attribute_descriptions(&attr_descs);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder().topology(vk::PrimitiveTopology::TRIANGLE_LIST).primitive_restart_enable(false);

    let viewport = vk::Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 };
    let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } };
    let viewports = [viewport];
    let scissors = [scissor];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder().viewports(&viewports).scissors(&scissors);

    let rasterization = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false)
        .line_width(1.0);

    let multisample = vk::PipelineMultisampleStateCreateInfo::builder().rasterization_samples(vk::SampleCountFlags::TYPE_1).sample_shading_enable(false);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState { blend_enable: vk::FALSE, color_write_mask: vk::ColorComponentFlags::RGBA, ..Default::default() };
    let color_blend_attachments = [color_blend_attachment];
    let color_blend = vk::PipelineColorBlendStateCreateInfo::builder().logic_op_enable(false).attachments(&color_blend_attachments);

    // D3D11 default: smaller depth value = nearer to the camera, depth
    // writes enabled, LESS comparison rejects fragments already occluded by
    // something nearer that was rasterized earlier in the same pass.
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false);

    let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder();
    let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_info, None) }.map_err(|e| GraphicsError::Vk("vkCreatePipelineLayout", e))?;

    let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisample)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&color_blend)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .build();
    let pipelines = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None) }
        .map_err(|(_, e)| GraphicsError::Vk("vkCreateGraphicsPipelines", e))?;
    let pipeline = pipelines[0];

    let readback_size = (width * height * 4) as vk::DeviceSize;
    let (readback_buffer, readback_memory) =
        create_host_visible_buffer(&device, &memory_properties, readback_size, vk::BufferUsageFlags::TRANSFER_DST)?;

    let pool_info = vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family_index);
    let command_pool = unsafe { device.create_command_pool(&pool_info, None) }.map_err(|e| GraphicsError::Vk("vkCreateCommandPool", e))?;
    let cmd_alloc_info = vk::CommandBufferAllocateInfo::builder().command_pool(command_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd_buffers = unsafe { device.allocate_command_buffers(&cmd_alloc_info) }.map_err(|e| GraphicsError::Vk("vkAllocateCommandBuffers", e))?;
    let cmd = cmd_buffers[0];

    let begin_info = vk::CommandBufferBeginInfo::builder();
    unsafe { device.begin_command_buffer(cmd, &begin_info) }.map_err(|e| GraphicsError::Vk("vkBeginCommandBuffer", e))?;

    let clear_values = [
        vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } },
        vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
    ];
    let render_pass_begin = vk::RenderPassBeginInfo::builder()
        .render_pass(render_pass)
        .framebuffer(framebuffer)
        .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width, height } })
        .clear_values(&clear_values);
    unsafe {
        device.cmd_begin_render_pass(cmd, &render_pass_begin, vk::SubpassContents::INLINE);
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
        device.cmd_bind_vertex_buffers(cmd, 0, &[vertex_buffer], &[0]);
        device.cmd_bind_index_buffer(cmd, index_buffer, 0, vk::IndexType::UINT32);
        device.cmd_draw_indexed(cmd, indices.len() as u32, 1, 0, 0, 0);
        device.cmd_end_render_pass(cmd);

        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D { width, height, depth: 1 },
        };
        device.cmd_copy_image_to_buffer(cmd, image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, readback_buffer, &[region]);
    }
    unsafe { device.end_command_buffer(cmd) }.map_err(|e| GraphicsError::Vk("vkEndCommandBuffer", e))?;

    let fence_info = vk::FenceCreateInfo::builder();
    let fence = unsafe { device.create_fence(&fence_info, None) }.map_err(|e| GraphicsError::Vk("vkCreateFence", e))?;
    let cmds = [cmd];
    let submit_info = vk::SubmitInfo::builder().command_buffers(&cmds).build();
    unsafe { device.queue_submit(queue, &[submit_info], fence) }.map_err(|e| GraphicsError::Vk("vkQueueSubmit", e))?;
    unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.map_err(|e| GraphicsError::Vk("vkWaitForFences", e))?;

    let pixels = unsafe {
        let ptr = device.map_memory(readback_memory, 0, readback_size, vk::MemoryMapFlags::empty()).map_err(|e| GraphicsError::Vk("vkMapMemory(readback)", e))?;
        let slice = std::slice::from_raw_parts(ptr as *const u8, readback_size as usize);
        let mut out = Vec::with_capacity((width * height) as usize);
        for chunk in slice.chunks_exact(4) {
            out.push(Rgba8 { r: chunk[0], g: chunk[1], b: chunk[2], a: chunk[3] });
        }
        device.unmap_memory(readback_memory);
        out
    };

    unsafe {
        device.destroy_fence(fence, None);
        device.destroy_command_pool(command_pool, None);
        device.destroy_pipeline(pipeline, None);
        device.destroy_pipeline_layout(pipeline_layout, None);
        device.destroy_shader_module(vs_module, None);
        device.destroy_shader_module(ps_module, None);
        device.destroy_framebuffer(framebuffer, None);
        device.destroy_render_pass(render_pass, None);
        device.destroy_image_view(image_view, None);
        device.destroy_image(image, None);
        device.free_memory(image_memory, None);
        device.destroy_image_view(depth_view, None);
        device.destroy_image(depth_image, None);
        device.free_memory(depth_memory, None);
        device.destroy_buffer(vertex_buffer, None);
        device.free_memory(vertex_buffer_memory, None);
        device.destroy_buffer(index_buffer, None);
        device.free_memory(index_buffer_memory, None);
        device.destroy_buffer(readback_buffer, None);
        device.free_memory(readback_memory, None);
    }
    drop(dguard);
    drop(guard);

    Ok(pixels)
}

fn create_host_visible_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { device.create_buffer(&buffer_info, None) }
        .map_err(|e| GraphicsError::Vk("vkCreateBuffer", e))?;
    let mem_req = unsafe { device.get_buffer_memory_requirements(buffer) };
    let mem_type = find_memory_type(
        memory_properties,
        mem_req.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )
    .ok_or(GraphicsError::NoGraphicsDevice)?;
    let alloc_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(mem_req.size)
        .memory_type_index(mem_type);
    let memory = unsafe { device.allocate_memory(&alloc_info, None) }
        .map_err(|e| GraphicsError::Vk("vkAllocateMemory(buffer)", e))?;
    unsafe { device.bind_buffer_memory(buffer, memory, 0) }
        .map_err(|e| GraphicsError::Vk("vkBindBufferMemory", e))?;
    Ok((buffer, memory))
}

fn find_memory_type(
    props: &vk::PhysicalDeviceMemoryProperties,
    type_bits: u32,
    required: vk::MemoryPropertyFlags,
) -> Option<u32> {
    (0..props.memory_type_count)
        .find(|&i| type_bits & (1 << i) != 0 && props.memory_types[i as usize].property_flags.contains(required))
}

struct InstanceGuard<'a> {
    instance: &'a ash::Instance,
}
impl Drop for InstanceGuard<'_> {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}

struct DeviceGuard<'a> {
    device: &'a ash::Device,
}
impl Drop for DeviceGuard<'_> {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None) };
    }
}
