//! This example shows how to render a simple triangle to the screen. This
//! is the most basic example, and it is useful to understand how the engine
//! works.
//! This is the most detailed example, to show all the steps needed to render
//! a triangle. In the other examples, not all steps are documented, especially
//! the ones that are already documented in this example.
pub use amethyst::prelude::*;
use std::sync::Arc;

/// The vertices of the triangle
static VERTICES: [Vertex2DColor; 3] = [
    Vertex2DColor {
        position: [0.0, -0.5],
        color: [0.0, 0.0, 1.0],
    },
    Vertex2DColor {
        position: [0.5, 0.5],
        color: [1.0, 0.0, 0.0],
    },
    Vertex2DColor {
        position: [-0.5, 0.5],
        color: [0.0, 1.0, 0.0],
    },
];

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
            title: "Amethyst triangle",
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
            front_face: FrontFace::Clockwise,
            ..Default::default()
        },
    );

    // Create a buffer to store the vertices of the triangle. We use a
    // pre-defined configuration to create the buffer to simplify the
    // example, but it is possible to create a buffer with a custom
    // configuration (see the `SubBufferCreateInfo` documentation for
    // more details)
    let vertices_buffer = SubBuffer::new(
        Arc::clone(&device),
        &VERTICES,
        BufferKind::Vertices,
        SubBufferCreateInfo::STATIC_RENDERING,
    );

    // Create a semaphore to make sure that the acquire operation of the
    // swapchain image is done before rendering to this image.
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
                // Acquire an image from the swapchain. This will block until an image
                // is available, and return the index of the image in the swapchain.
                // From this index, we can get the image view and the image itself.
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

                // Record the command buffer. The recording is done each frame, but in
                // this case, it could be done only once before the main loop because
                // the command buffer is pretty much static and does not change across
                // frames.
                let command = command
                    .start_recording()
                    // We must transition the image format manually, because the image acquired
                    // from the swapchain is not guaranteed to have the format that we want during
                    // the rendering. We must transition it to the `ColorAttachmentOptimal` layout
                    // to correctly render to it.
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
                    // Start the rendering inside the image given by the swapchain.
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
                    // Draw the triangle using the vertices buffer.
                    .draw(DrawCommandInfo {
                        vertex_count: VERTICES.len() as u32,
                        instance_count: 1,
                        first_instance: 0,
                        first_vertex: 0,
                    })
                    .end_rendering()
                    // Here, must must again transition the image format: this time,
                    // the image has the ColorAttachmentOptimal layout, but in order
                    // to be able to present the image to the swapchain, it must be
                    // in the PresentSrcKhr layout. So we transition again the image
                    // to this layout.
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
