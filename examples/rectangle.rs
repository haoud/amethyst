//! This example shows how to render a simple rectangle to the screen. It is
//! very similar to the triangle example, but it uses a indices buffer to
//! reduce the number of vertices needed to draw the rectangle and therefore,
//! the number of memory used to render it. For this example, it is pretty
//! useless, but it is very useful when rendering big and complex objects.
pub use amethyst::prelude::*;
use std::sync::Arc;

/// The vertices of the rectangle
static VERTICES: [Vertex2DColor; 4] = [
    Vertex2DColor {
        position: [-0.5, -0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex2DColor {
        position: [0.5, -0.5],
        color: [0.0, 1.0, 0.0],
    },
    Vertex2DColor {
        position: [0.5, 0.5],
        color: [0.0, 0.0, 1.0],
    },
    Vertex2DColor {
        position: [-0.5, 0.5],
        color: [1.0, 1.0, 1.0],
    },
];

/// The indices of the rectangle
static INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

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
            title: "Amethyst rectangle",
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

    let pipeline = Pipeline::new::<Vertex2DColor>(
        Arc::clone(&device),
        &swapchain,
        PipelineCreateInfo {
            shaders: vec![
                Shader::compile(
                    Arc::clone(&device),
                    ShaderCompileInfo {
                        language: ShaderSourceType::GLSL,
                        source: ShaderSource::File("examples/shaders/simple.vert"),
                        kind: ShaderType::Vertex,
                        ..Default::default()
                    },
                ),
                Shader::compile(
                    Arc::clone(&device),
                    ShaderCompileInfo {
                        language: ShaderSourceType::GLSL,
                        source: ShaderSource::File("examples/shaders/simple.frag"),
                        kind: ShaderType::Fragment,
                        ..Default::default()
                    },
                ),
            ],
            front_face: FrontFace::Clockwise,
            ..Default::default()
        },
    );

    // Create a buffer to store the vertices of the rectangle.
    let vertices_buffer = SubBuffer::new(
        Arc::clone(&device),
        &VERTICES,
        SubBufferCreateInfo {
            usage: BufferUsageInfo::STATIC_RENDERING,
            kind: BufferKind::Vertices,
            ..Default::default()
        },
    );

    // Create a buffer to store the indices of the rectangle.
    let indices_buffer = SubBuffer::new(
        Arc::clone(&device),
        &INDICES,
        SubBufferCreateInfo {
            usage: BufferUsageInfo::STATIC_RENDERING,
            kind: BufferKind::Indices,
            ..Default::default()
        },
    );

    let acquire_semaphore = Semaphore::new(Arc::clone(&device));
    let render_semaphore = Semaphore::new(Arc::clone(&device));

    window.run(engine, move |_, event| {
        match event {
            WindowEvent::MainLoop => {
                let image_index = swapchain.acquire_image_index(&acquire_semaphore);
                let image_view = &swapchain.images_views()[image_index as usize];
                let image = &swapchain.images()[image_index as usize];

                let command = Command::new(
                    Arc::clone(&device),
                    CommandCreateInfo {
                        ..Default::default()
                    },
                );

                let command = command
                    .start_recording()
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
                    // Bind the indices buffer to the pipeline. This is needed
                    // to call the `draw_indexed` method. When binding the indices
                    // buffer, we need to specify the type of the indices.
                    .bind_indices_buffers(&indices_buffer, IndicesType::U16)
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
                    // Draw the triangle. Instread of using the `draw` method, we
                    // use the `draw_indexed` method to use the indices buffer. To
                    // do so, we need to specify the number of indices to draw and
                    // the first index to use.
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
