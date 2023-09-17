//! This example shows how to create a rotating 2D triangle using Amethyst.
pub use amethyst::prelude::*;
use std::sync::Arc;

/// The uniform object that will be passed to the vertex shader. It MUST
/// use the `repr(C)` attribute to have a fixed layout in memory and be
/// compatible with the shader. When creating an uniform buffer, you
/// must also take care of the alignment of the data that the shader
/// expects.
#[derive(Debug, Clone)]
#[repr(C)]
struct UniformData {
    projection: glm::Mat4,
    model: glm::Mat4,
    view: glm::Mat4,
}

/// The vertices of the triangle
static VERTICES: [Vertex3DColor; 3] = [
    Vertex3DColor {
        position: [0.0, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
    Vertex3DColor {
        position: [0.5, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex3DColor {
        position: [-0.5, 0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
];

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format_timestamp(None)
        .format_target(false)
        .init();

    let mut engine = Engine::new(EngineInfo {
        ..Default::default()
    });

    let window = Window::new(
        &mut engine,
        WindowInfo {
            title: "Amethyst rotating triangle",
            ..Default::default()
        },
    );

    let vulkan = Arc::new(Vulkan::new(
        &mut engine,
        VulkanInfo {
            enable_validation: true,
            window: Some(&window),
            ..Default::default()
        },
    ));

    let device = Arc::new(RenderDevice::pick_best(
        vulkan,
        RenderDevicePickInfo {
            ..Default::default()
        },
    ));

    let swapchain = Swapchain::new(
        &window,
        Arc::clone(&device),
        SwapchainCreatInfo {
            present_mode: PresentMode::Vsync,
            ..Default::default()
        },
    );

    // Get the height and width of the swapchain. This should be the same
    // as the window's height and width, but it may be different in some
    // cases.
    let height = swapchain.extent().height as f32;
    let width = swapchain.extent().width as f32;

    // Create a descriptor set layout to describe the data that will be
    // passed to the shaders. Here, we only pass one uniform buffer, so
    // we create a descriptor set layout with only one binding.
    let descriptor_layout = DescriptorSetLayout::new(
        Arc::clone(&device),
        &[DescriptorSetLayoutBinding {
            descriptor_type: DescriptorType::Uniform,
            shader_stages: ShaderStages::VERTEX,
            binding: 0,
        }],
    );

    let pipeline = Pipeline::new::<Vertex3DColor>(
        Arc::clone(&device),
        &swapchain,
        PipelineCreateInfo {
            shaders: vec![
                Shader::compile(
                    Arc::clone(&device),
                    ShaderCompileInfo {
                        language: ShaderSourceType::GLSL,
                        source: ShaderSource::File("examples/shaders/rotating.vert"),
                        kind: ShaderType::Vertex,
                        ..Default::default()
                    },
                ),
                Shader::compile(
                    Arc::clone(&device),
                    ShaderCompileInfo {
                        language: ShaderSourceType::GLSL,
                        source: ShaderSource::File("examples/shaders/rotating.frag"),
                        kind: ShaderType::Fragment,
                        ..Default::default()
                    },
                ),
            ],

            // The descriptor set layout is used to describe the data that
            // will be passed to the shaders. Here, we only pass the uniform
            // data, so we only need one descriptor set layout.
            descriptor_set_layouts: vec![descriptor_layout],

            // Disable the culling of the back faces of the triangle. This
            // is necessary because the triangle is rotating, and we want
            // to see all the faces
            cull_mode: CullMode::None,
            ..Default::default()
        },
    );

    // Create a camera. The camera is used to create the view and projection
    // matrices that will be passed to the shaders.
    let camera = Camera::new(CameraCreateInfo {
        direction: glm::vec3(0.0, 0.0, 0.0),
        position: glm::vec3(1.0, 1.0, 1.0),
        height: height,
        width: width,
        ..Default::default()
    });

    // Create a buffer to store the uniform data.
    let uniform_buffer = SubBuffer::new(
        Arc::clone(&device),
        &[UniformData {
            projection: camera.projection().clone(),
            model: glm::identity(),
            view: camera.projection().clone(),
        }],
        SubBufferCreateInfo {
            usage: BufferUsageInfo::UNIFORM,
            kind: BufferKind::Uniforms,
            ..Default::default()
        },
    );

    // Create a buffer to store the vertices of the triangle.
    let vertices_buffer = SubBuffer::new(
        Arc::clone(&device),
        &VERTICES,
        SubBufferCreateInfo {
            usage: BufferUsageInfo::STATIC_RENDERING,
            kind: BufferKind::Vertices,
            ..Default::default()
        },
    );

    // Create a descriptor pool to allocate the descriptor sets.
    let descriptor_pool = Arc::new(DescriptorPool::new(
        Arc::clone(&device),
        &[DescriptorPoolCreateInfo {
            descriptor_type: DescriptorType::Uniform,
            descriptor_count: swapchain.images().len() as u32,
            ..Default::default()
        }],
    ));

    // Create and update the descriptor set with the new uniform buffer.
    let descriptor_set = DescriptorSet::new(Arc::clone(&device), descriptor_pool, &pipeline);
    descriptor_set.update_buffer(0, &uniform_buffer);

    let acquire_semaphore = Semaphore::new(Arc::clone(&device));
    let render_semaphore = Semaphore::new(Arc::clone(&device));

    let start = std::time::Instant::now();

    window.run(engine, move |_, event| {
        match event {
            WindowEvent::MainLoop => {
                let image_index = swapchain.acquire_image_index(&acquire_semaphore);
                let image_view = &swapchain.images_views()[image_index as usize];
                let image = &swapchain.images()[image_index as usize];

                // Rotate the triangle around the Y axis.
                let model = glm::rotate(
                    &glm::identity(),
                    start.elapsed().as_secs_f32() * glm::radians(&glm::vec1(90.0))[0],
                    &glm::vec3(0.0, 1.0, 0.0),
                );

                let command = Command::new(
                    Arc::clone(&device),
                    CommandCreateInfo {
                        ..Default::default()
                    },
                );

                command
                    .start_recording()
                    // Update the uniform buffer with the new model matrix.
                    .update_buffer(
                        &uniform_buffer,
                        &[UniformData {
                            projection: camera.projection().clone(),
                            model: model,
                            view: camera.view(),
                        }],
                    )
                    .pipeline_barrier(PipelineBarrierInfo {
                        src_stage_mask: PipelineStage::TOP_OF_PIPE,
                        dst_stage_mask: PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                        images_barriers: vec![ImageBarrier {
                            subresource_range: ImageSubResourceRange::default(),
                            src_access_mask: ImageAccess::UNDEFINED,
                            dst_access_mask: ImageAccess::COLOR_ATTACHMENT_WRITE,
                            old_layout: ImageLayout::Undefined,
                            new_layout: ImageLayout::ColorAttachmentOptimal,
                            image: image,
                        }],
                    })
                    .bind_graphics_pipeline(&pipeline)
                    .bind_vertex_buffers(&vertices_buffer)
                    .bind_descriptor_sets(&pipeline, &[&descriptor_set])
                    .start_rendering(RenderingInfo {
                        render_area: swapchain.extent(),
                        colors_attachements: vec![RenderingAttachementInfo {
                            image_view: image_view,
                            image_layout: ImageLayout::AttachmentOptimal,
                            load_op: AttachmentLoadOp::Clear,
                            store_op: AttachmentStoreOp::Store,
                            clear_value: ClearValue::Color([0.0, 0.0, 0.0, 1.0]),
                        }],
                        depth_attachement: None,
                    })
                    .draw(DrawCommandInfo {
                        vertex_count: VERTICES.len() as u32,
                        instance_count: 1,
                        first_instance: 0,
                        first_vertex: 0,
                    })
                    .end_rendering()
                    .pipeline_barrier(PipelineBarrierInfo {
                        src_stage_mask: PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                        dst_stage_mask: PipelineStage::BOTTOM_OF_PIPE,
                        images_barriers: vec![ImageBarrier {
                            subresource_range: ImageSubResourceRange::default(),
                            src_access_mask: ImageAccess::COLOR_ATTACHMENT_WRITE,
                            dst_access_mask: ImageAccess::UNDEFINED,
                            old_layout: ImageLayout::ColorAttachmentOptimal,
                            new_layout: ImageLayout::PresentSrcKhr,
                            image: image,
                        }],
                    })
                    .stop_recording()
                    .submit_to(
                        device.graphic_queue(),
                        CommandSubmitInfo {
                            signal_semaphore: &[&render_semaphore],
                            wait_semaphore: &[&acquire_semaphore],
                        },
                    );

                // Present the image to the swapchain.
                swapchain.present_image(SwapchainPresentInfo {
                    wait_semaphore: &render_semaphore,
                    image_index,
                });

                Status::Continue
            }

            WindowEvent::LoopDestroyed => {
                device.logical().wait_idle();
                Status::Exit
            }

            WindowEvent::Exit => Status::Exit,
        }
    })
}
