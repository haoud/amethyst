//! This example shows how to draw a textured rectangle by loading an image
//! from the disk and using it as a texture.
pub use amethyst::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
#[repr(C)]
struct UniformData {
    projection: glm::Mat4,
    model: glm::Mat4,
    view: glm::Mat4,
}

/// The vertices of the rectangle
static VERTICES: [Vertex3DTexture2D; 4] = [
    Vertex3DTexture2D {
        position: [-0.5, -0.5, 0.0],
        texture: [1.0, 0.0],
    },
    Vertex3DTexture2D {
        position: [0.5, -0.5, 0.0],
        texture: [0.0, 0.0],
    },
    Vertex3DTexture2D {
        position: [0.5, 0.5, 0.0],
        texture: [0.0, 1.0],
    },
    Vertex3DTexture2D {
        position: [-0.5, 0.5, 0.0],
        texture: [1.0, 1.0],
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
            title: "Amethyst textured rectangle",
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

    let descriptor_layout = DescriptorSetLayout::new(
        Arc::clone(&device),
        &[
            // The uniform buffer, located at binding 0 and used by the
            // vertex shader.
            DescriptorSetLayoutBinding {
                descriptor_type: DescriptorType::Uniform,
                shader_stages: ShaderStages::VERTEX,
                binding: 0,
            },
            // The texture sampler, located at binding 1 and used by the
            // fragment shader.
            DescriptorSetLayoutBinding {
                descriptor_type: DescriptorType::Sampler,
                shader_stages: ShaderStages::FRAGMENT,
                binding: 1,
            },
        ],
    );

    let pipeline = Pipeline::new::<Vertex3DTexture2D>(
        Arc::clone(&device),
        &swapchain,
        PipelineCreateInfo {
            shaders: vec![
                Shader::compile(
                    Arc::clone(&device),
                    ShaderCompileInfo {
                        language: ShaderSourceType::GLSL,
                        source: ShaderSource::File("examples/shaders/texture.vert"),
                        kind: ShaderType::Vertex,
                        ..Default::default()
                    },
                ),
                Shader::compile(
                    Arc::clone(&device),
                    ShaderCompileInfo {
                        language: ShaderSourceType::GLSL,
                        source: ShaderSource::File("examples/shaders/texture.frag"),
                        kind: ShaderType::Fragment,
                        ..Default::default()
                    },
                ),
            ],

            descriptor_set_layouts: vec![descriptor_layout],
            cull_mode: CullMode::None,
            ..Default::default()
        },
    );

    let camera = Camera::new(CameraCreateInfo {
        direction: glm::vec3(0.0, 0.0, 0.0),
        position: glm::vec3(0.5, 0.5, 0.5),
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

    let (image, width, height) = load_image();
    let texture = Image::new(
        Arc::clone(&device),
        ImageCreateInfo {
            usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
            format: ImageFormat::R8G8B8A8SRGB,
            extent: Extent2D {
                height: height,
                width: width,
            },
            data: &image,
        },
    );

    let texture_view = ImageView::new(
        Arc::clone(&device),
        &texture,
        ImageViewCreateInfo {
            subresource: ImageSubResourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_array_layer: 0,
                base_mip_level: 0,
                level_count: 1,
                layer_count: 1,
            },
            format: ImageFormat::R8G8B8A8SRGB,
            kind: ImageViewKind::Type2D,
        },
    );

    let texture_sampler = ImageSampler::new(Arc::clone(&device), ImageSamplerCreatInfo {});

    // Create a descriptor pool to allocate the descriptor sets, and create
    // a descriptor set from the descriptor pool with the uniform data buffer.
    let descriptor_pool = Arc::new(DescriptorPool::new(
        Arc::clone(&device),
        &[
            DescriptorPoolCreateInfo {
                descriptor_count: swapchain.images().len() as u32,
                descriptor_type: DescriptorType::Uniform,
            },
            DescriptorPoolCreateInfo {
                descriptor_count: swapchain.images().len() as u32,
                descriptor_type: DescriptorType::Sampler,
            },
        ],
    ));

    let descriptor_set = DescriptorSet::new(Arc::clone(&device), descriptor_pool, &pipeline);

    descriptor_set.update_buffer(0, &uniform_buffer);
    descriptor_set.update_image(
        1,
        ImageDescriptorInfo {
            layout: ImageLayout::ShaderReadOnlyOptimal,
            sampler: &texture_sampler,
            view: &texture_view,
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
                    // Update the uniform buffer with the new model matrix.
                    .update_buffer(
                        &uniform_buffer,
                        &[UniformData {
                            projection: camera.projection().clone(),
                            model: glm::identity(),
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
                        depth_attachement: None,
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

/// Loads the image from the disk and returns the pixels, width and height.
fn load_image() -> (Vec<u8>, u32, u32) {
    let file =
        std::fs::File::open("examples/resources/texture.png").expect("Failed to open PNG file");

    let decoder = png::Decoder::new(file);
    let mut reader = decoder
        .read_info()
        .expect("Failed to read PNG info");

    let mut pixels = vec![0; reader.info().raw_bytes()];

    reader
        .next_frame(&mut pixels)
        .expect("Failed to read PNG pixels");

    let (width, height) = reader.info().size();
    (pixels, width, height)
}
