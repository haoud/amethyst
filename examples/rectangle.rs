//! This example shows how to render a simple rectangle to the screen. It is
//! very similar to the triangle example, but it uses a indices buffer to
//! reduce the number of vertices needed to draw the rectangle.
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
    // Configure logger to print everything. This is useful for debugging, and can
    // be adjusted to filter out less important messages. If enabled, validation
    // layers will print messages through the logger.
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format_timestamp(None)
        .format_target(false)
        .init();

    // Initialize the engine with our own custom configuration. For now, we
    // just use the default configuration (only a few things are configurable
    // for now anyway)
    let mut engine = Engine::new(EngineInfo {
        ..Default::default()
    });

    // Create a new window. This must be created before the Vulkan instance
    // because the Vulkan instance need some information about the window
    // to load the required extensions and create the associated surface.
    let window = Window::new(
        &mut engine,
        WindowInfo {
            title: "Amethyst rectangle",
            ..Default::default()
        },
    );

    // Create a new Vulkan instance. This will load and configure the Vulkan
    // driver used by the application. Here, we provide a window to load the
    // Vulkan extensions required for rendering to a window, and enable the
    // validation layers to catch Vulkan API misuse during development.
    // If a window is provided, it will also be used to create the surface
    // where the image will be transferred after the rendering is done.
    let vulkan = Arc::new(Vulkan::new(
        &mut engine,
        VulkanInfo {
            enable_validation: true,
            window: Some(&window),
            ..Default::default()
        },
    ));

    // Pick the best physical device available that matches all the engine's
    // requirements and create a logical device from it. This is not
    // configurable for now, but it should be fine for most use cases.
    let device = Arc::new(RenderDevice::pick_best(
        vulkan,
        RenderDevicePickInfo {
            ..Default::default()
        },
    ));

    // Create the swapchain to allow rendering to the window.
    let swapchain = Swapchain::new(
        &window,
        Arc::clone(&device),
        SwapchainCreatInfo {
            present_mode: PresentMode::Vsync,
            ..Default::default()
        },
    );

    // Create a graphic pipeline to render the image. It is the most
    // complex object to create, but it is also the most important one.
    // It contains the vertex and fragment shaders and the render pass
    // that will be used to render the image to the window.
    let pipeline = Pipeline::new::<Vertex2DColor>(
        Arc::clone(&device),
        &swapchain,
        PipelineCreateInfo {
            // More shaders can be added to the pipeline if needed.
            shaders: vec![
                // Compile the vertex shader
                Shader::compile(
                    Arc::clone(&device),
                    ShaderCompileInfo {
                        language: ShaderSourceType::GLSL,
                        source: ShaderSource::File("examples/shaders/simple.vert"),
                        kind: ShaderType::Vertex,
                        ..Default::default()
                    },
                ),
                // Compile the fragment shader
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
            ..Default::default()
        },
    );

    // Create a buffer to store the vertices of the rectangle.
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

    // Create a semaphore to make sure that the acquire operation is done
    // before rendering to the image.
    let acquire_semaphore = Semaphore::new(Arc::clone(&device));

    // Create a semaphore to make sure that the rendering is done before
    // presenting the image.
    let render_semaphore = Semaphore::new(Arc::clone(&device));

    // Please note that all variables not used in the closure or not registered
    // as resources will NOT be dropped when the closure will return `Status::Exit`.
    // This is a limitation of the window subsystem used by Amethyst, and there
    // is no way to fix it at the moment.
    window.run(engine, move |_, event| {
        match event {
            WindowEvent::MainLoop => {
                let image_index = swapchain.acquire_image_index(&acquire_semaphore);
                let image_view = &swapchain.images_views()[image_index as usize];
                let image = &swapchain.images()[image_index as usize];

                // Create a new command buffer from the command pool.
                let command = Command::new(
                    Arc::clone(&device),
                    CommandCreateInfo {
                        ..Default::default()
                    },
                );

                // Record the command buffer.
                let command = command
                    .start_recording()
                    // We must transition the image format manually, because the image acquired
                    // from the swapchain is not guaranteed to have the format that we want.
                    // Here, we want to render to the image, so we must transition it to the
                    // `ColorAttachmentOptimal` layout.
                    .pipeline_barrier(PipelineBarrierInfo {
                        src_stage_mask: PipelineStage::TOP_OF_PIPE,
                        dst_stage_mask: PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                        images_barriers: vec![ImageBarrier {
                            src_access_mask: ImageAccess::UNDEFINED,
                            dst_access_mask: ImageAccess::COLOR_ATTACHMENT_WRITE,
                            old_layout: ImageLayout::Undefined,
                            new_layout: ImageLayout::ColorAttachmentOptimal,
                            image: image,
                        }],
                    })
                    .bind_graphics_pipeline(&pipeline)
                    .bind_vertex_buffers(&vertices_buffer)
                    .bind_indices_buffers(&indices_buffer, IndicesType::U16)
                    // Start the rendering inside the image given by the
                    // swapchain.
                    .start_rendering(RenderingInfo {
                        render_area: swapchain.extent(),
                        colors_attachements: vec![RenderingAttachementInfo {
                            image_view: image_view,
                            image_layout: ImageLayout::AttachmentOptimal,
                            load_op: AttachmentLoadOp::Clear,
                            store_op: AttachmentStoreOp::Store,
                            clear_color: [0.0, 0.0, 0.0, 1.0],
                        }],
                    })
                    // Draw the triangle.
                    .draw_indexed(DrawIndexedCommandInfo {
                        index_count: INDICES.len() as u32,
                        instance_count: 1,
                        first_instance: 0,
                        first_index: 0,
                    })
                    .end_rendering()
                    // Here, must must again transition the image format: this time,
                    // the image has the ColorAttachmentOptimal layout, but in order
                    // to be able to present the image to the swapchain, it must be
                    // in the PresentSrcKhr layout. So we transition the image to
                    // this layout.
                    .pipeline_barrier(PipelineBarrierInfo {
                        src_stage_mask: PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                        dst_stage_mask: PipelineStage::BOTTOM_OF_PIPE,
                        images_barriers: vec![ImageBarrier {
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
                // Wait for the rendering to be done before exiting.
                // This is necessary to avoid destroying resources
                // that are still in use.
                device.logical().wait_idle();
                Status::Exit
            }

            WindowEvent::Exit => Status::Exit,
        }
    })
}
