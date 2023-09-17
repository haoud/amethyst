//! This example shows how to render a 3D cube with Amethyst. It use many
//! different features of Amethyst, but the good news is that you have
//! touched the most important features of Amethyst, and you can now start
//! to create your own application.
pub use amethyst::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
#[repr(C)]
struct UniformData {
    projection: glm::Mat4,
    model: glm::Mat4,
    view: glm::Mat4,
}

/// The vertices of the cube. We need to duplicate the vertices because
/// each vertex can have only one color. If we don't duplicate them, it
/// will be impossible to have a cube with different colors on each face,
/// and the cube will have faces with a ugly gradient color (trust me).
static VERTICES: [Vertex3DColor; 24] = [
    // Front face
    Vertex3DColor {
        position: [-0.5, -0.5, -0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex3DColor {
        position: [0.5, -0.5, -0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex3DColor {
        position: [0.5, 0.5, -0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex3DColor {
        position: [-0.5, 0.5, -0.5],
        color: [1.0, 0.0, 0.0],
    },
    // Back face
    Vertex3DColor {
        position: [-0.5, -0.5, 0.5],
        color: [0.0, 1.0, 0.0],
    },
    Vertex3DColor {
        position: [0.5, -0.5, 0.5],
        color: [0.0, 1.0, 0.0],
    },
    Vertex3DColor {
        position: [0.5, 0.5, 0.5],
        color: [0.0, 1.0, 0.0],
    },
    Vertex3DColor {
        position: [-0.5, 0.5, 0.5],
        color: [0.0, 1.0, 0.0],
    },
    // Top face
    Vertex3DColor {
        position: [-0.5, 0.5, -0.5],
        color: [0.0, 0.0, 1.0],
    },
    Vertex3DColor {
        position: [0.5, 0.5, -0.5],
        color: [0.0, 0.0, 1.0],
    },
    Vertex3DColor {
        position: [0.5, 0.5, 0.5],
        color: [0.0, 0.0, 1.0],
    },
    Vertex3DColor {
        position: [-0.5, 0.5, 0.5],
        color: [0.0, 0.0, 1.0],
    },
    // Bottom face
    Vertex3DColor {
        position: [-0.5, -0.5, -0.5],
        color: [1.0, 1.0, 0.0],
    },
    Vertex3DColor {
        position: [0.5, -0.5, -0.5],
        color: [1.0, 1.0, 0.0],
    },
    Vertex3DColor {
        position: [0.5, -0.5, 0.5],
        color: [1.0, 1.0, 0.0],
    },
    Vertex3DColor {
        position: [-0.5, -0.5, 0.5],
        color: [1.0, 1.0, 0.0],
    },
    // Right face
    Vertex3DColor {
        position: [0.5, -0.5, -0.5],
        color: [0.0, 1.0, 1.0],
    },
    Vertex3DColor {
        position: [0.5, 0.5, -0.5],
        color: [0.0, 1.0, 1.0],
    },
    Vertex3DColor {
        position: [0.5, 0.5, 0.5],
        color: [0.0, 1.0, 1.0],
    },
    Vertex3DColor {
        position: [0.5, -0.5, 0.5],
        color: [0.0, 1.0, 1.0],
    },
    // Left face
    Vertex3DColor {
        position: [-0.5, -0.5, -0.5],
        color: [1.0, 0.0, 1.0],
    },
    Vertex3DColor {
        position: [-0.5, 0.5, -0.5],
        color: [1.0, 0.0, 1.0],
    },
    Vertex3DColor {
        position: [-0.5, 0.5, 0.5],
        color: [1.0, 0.0, 1.0],
    },
    Vertex3DColor {
        position: [-0.5, -0.5, 0.5],
        color: [1.0, 0.0, 1.0],
    },
];

static INDICES: [u16; 36] = [
    0, 1, 2, 2, 3, 0, // Front face
    4, 5, 6, 6, 7, 4, // Back face
    8, 9, 10, 10, 11, 8, // Top face
    12, 13, 14, 14, 15, 12, // Bottom face
    16, 17, 18, 18, 19, 16, // Right face
    20, 21, 22, 22, 23, 20, // Left face
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
            title: "Amethyst 3D cube",
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

    let height = swapchain.extent().height as f32;
    let width = swapchain.extent().width as f32;

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

            descriptor_set_layouts: vec![descriptor_layout],

            /// Disable the culling because the cube has some clockwise and
            /// counter-clockwise faces, and I'm too lazy to fix it now.
            cull_mode: CullMode::None,

            /// The format of the depth buffer. Theoricaly, we should to verify
            /// if this format is supported by the device, but this format is
            /// almost always supported by Vulkan implementations.
            depth_format: ImageFormat::D32SFLOAT,

            // Enable writing to the depth buffer when rendering.
            depth_write: true,

            // Enable depth testing. The depth test is used to discard the
            // fragments that are behind other fragments.
            depth_test: true,

            ..Default::default()
        },
    );

    // Create the depth buffer. The depth buffer (or depth image) has the
    // same size as the swapchain images. We don't need to provide data
    // because we will never read from the depth buffer before writing to
    // it.
    let depth_image = Image::new(
        Arc::clone(&device),
        ImageCreateInfo {
            usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
            format: ImageFormat::D32SFLOAT,
            extent: swapchain.extent(),
            data: &[],
        },
    );

    // Create the depth buffer view. The depth buffer view is used to
    // describe how to access the depth image.
    let depth_view = ImageView::new(
        Arc::clone(&device),
        &depth_image,
        ImageViewCreateInfo {
            subresource: ImageSubResourceRange {
                aspect_mask: ImageAspectFlags::DEPTH,
                ..Default::default()
            },
            format: ImageFormat::D32SFLOAT,
            kind: ImageViewKind::Type2D,
        },
    );

    let camera = Camera::new(CameraCreateInfo {
        direction: glm::vec3(0.0, 0.0, 0.0),
        position: glm::vec3(1.0, 1.0, 1.0),
        height: height,
        width: width,
        ..Default::default()
    });

    let uniform_buffer = SubBuffer::new(
        Arc::clone(&device),
        &[UniformData {
            projection: camera.projection().clone(),
            model: glm::identity(),
            view: camera.projection().clone(),
        }],
        BufferKind::Uniforms,
        SubBufferCreateInfo::UNIFORM,
    );

    let vertices_buffer = SubBuffer::new(
        Arc::clone(&device),
        &VERTICES,
        BufferKind::Vertices,
        SubBufferCreateInfo::STATIC_RENDERING,
    );

    let indices_buffer = SubBuffer::new(
        Arc::clone(&device),
        &INDICES,
        BufferKind::Indices,
        SubBufferCreateInfo::STATIC_RENDERING,
    );

    let descriptor_pool = Arc::new(DescriptorPool::new(
        Arc::clone(&device),
        &[DescriptorPoolCreateInfo {
            descriptor_type: DescriptorType::Uniform,
            descriptor_count: 32,
            ..Default::default()
        }],
    ));

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

                let model = glm::rotate(
                    &glm::identity(),
                    start.elapsed().as_secs_f32() * 0.5 * glm::radians(&glm::vec1(90.0))[0],
                    &glm::vec3(1.0, 1.0, 0.0),
                );

                let command = Command::new(
                    Arc::clone(&device),
                    CommandCreateInfo {
                        ..Default::default()
                    },
                );

                let command = command
                    .start_recording()
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
                    // Transition the depth buffer from an undefined layout to a
                    // depth attachment layout, and specify that we will read and
                    // write to the depth buffer during the EARLY_FRAGMENT_TESTS
                    // pipeline stage.
                    .pipeline_barrier(PipelineBarrierInfo {
                        src_stage_mask: PipelineStage::TOP_OF_PIPE,
                        dst_stage_mask: PipelineStage::EARLY_FRAGMENT_TESTS,
                        images_barriers: vec![ImageBarrier {
                            subresource_range: ImageSubResourceRange {
                                aspect_mask: ImageAspectFlags::DEPTH,
                                ..Default::default()
                            },
                            src_access_mask: ImageAccess::UNDEFINED,
                            dst_access_mask: ImageAccess::DEPTH_STENCIL_ATTACHMENT_READ
                                | ImageAccess::DEPTH_STENCIL_ATTACHMENT_WRITE,
                            old_layout: ImageLayout::Undefined,
                            new_layout: ImageLayout::DepthStencilAttachmentOptimal,
                            image: &depth_image,
                        }],
                    })
                    .bind_graphics_pipeline(&pipeline)
                    .bind_vertex_buffers(&vertices_buffer)
                    .bind_indices_buffers(&indices_buffer, IndicesType::U16)
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
                        // Attach the depth buffer to the rendering. The depth buffer
                        // will be cleared before the rendering with the specified clear
                        // value below (1.0).
                        depth_attachement: Some(RenderingAttachementInfo {
                            image_view: &depth_view,
                            image_layout: ImageLayout::DepthStencilAttachmentOptimal,
                            load_op: AttachmentLoadOp::Clear,
                            store_op: AttachmentStoreOp::Discard,
                            clear_value: ClearValue::DepthStencil(1.0, 0),
                        }),
                    })
                    .draw_indexed(DrawIndexedCommandInfo {
                        index_count: INDICES.len() as u32,
                        instance_count: 1,
                        first_instance: 0,
                        first_index: 0,
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
                    .stop_recording();

                // Submit the command buffer to the graphic queue.
                device.graphic_queue().submit(
                    &device,
                    QueueSubmitInfo {
                        signal_semaphore: &[&render_semaphore],
                        wait_semaphore: &[&acquire_semaphore],
                        commands: &[&command],
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
