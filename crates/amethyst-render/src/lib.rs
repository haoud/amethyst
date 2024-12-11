use amethyst_vulkan::{
    buffer::{
        Buffer, BufferAccess, BufferAllocator, BufferCreateInfo, BufferDataInfo,
        BufferMemoryLocation, BufferTransfert, BufferUsage, BufferUsageInfo,
    },
    command::{
        CommandBuffer, CommandPool, DrawInfo, PipelineBarrierInfo, RenderingInfo, SubmitInfo,
    },
    context::VulkanContext,
    device::{VulkanDevice, VulkanQueues},
    pipeline::{Pipeline, PipelineCreateInfo},
    semaphore::Semaphore,
    shader::{ShaderModule, ShaderType},
    swapchain::{Surface, VulkanSwapchain},
};
use bevy::{
    prelude::*,
    window::{PrimaryWindow, RawHandleWrapperHolder},
};
use std::sync::Arc;
use vertex::Vertex2DColor;
use vulkanalia::prelude::v1_3::*;

pub mod vertex;

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

#[derive(Debug)]
pub struct AmethystRender;

impl Plugin for AmethystRender {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_vulkan_context);
        app.add_systems(Update, render);
        app.add_systems(PostUpdate, wait_for_device.run_if(is_exiting));
    }
}

/// # Important
/// The order of fields in the struct is important to ensure that the resources are dropped in the
/// right order. The fields are dropped in the order they are defined in the struct, and some fields
/// must be destroyed before others. Please keep this in mind when adding new fields to the struct.
#[allow(dead_code)]
#[derive(Debug, Resource)]
pub struct Render {
    /// A vertex buffer that holds the vertices of the triangle
    buffer: Buffer,

    /// A buffer allocator used to allocate buffers
    buffer_allocator: Arc<BufferAllocator>,

    /// A semaphore used to signal when the swapchain image is acquired
    acquire_semaphore: Semaphore,

    /// A semaphore used to signal when the rendering is done
    render_semaphore: Semaphore,

    /// A simple pipeline object that renders a triangle1
    pipeline: Pipeline,

    /// The swapchain used for presenting images to the screen
    swapchain: VulkanSwapchain,

    /// The queues used for rendering
    queues: VulkanQueues,

    /// The device used for rendering
    device: Arc<VulkanDevice>,

    /// The Vulkan context object that holds the Vulkan instance
    context: Arc<VulkanContext>,
}

fn create_vulkan_context(
    mut command: Commands,
    window: Query<&RawHandleWrapperHolder, With<PrimaryWindow>>,
) {
    let handle = window
        .get_single()
        .expect("No primary window found")
        .0
        .lock()
        .expect("Could not lock primary window handle")
        .as_ref()
        .expect("Vulkan plugin requires a window to work correctly")
        .clone();

    // SAFETY: Adding plugin to the app should be done in the main thread,
    // so we can safely get the handle in any platform.
    let handle = unsafe { handle.get_handle() };

    // Create the Vulkan context and surface objects
    let context = Arc::new(VulkanContext::new(&handle));
    let surface = Surface::new(context.clone(), handle);

    // Create the device, swapchain, and queues objects
    let device = Arc::new(VulkanDevice::pick_best(&context, &surface));
    let swapchain = VulkanSwapchain::new(context.clone(), device.clone(), surface);
    let queues = VulkanQueues::fetch(&device);

    // Create a pipeline object that does not require vertex data and
    // use a simple vertex and fragment shader. Since we are trying to
    // render a simple triangle, we don't need to pass any vertex data
    // to the vertex shader (hence the `NoVertex` type) and we also don't
    // need to write to the depth buffer.
    let pipeline = Pipeline::new::<Vertex2DColor>(
        device.clone(),
        &swapchain,
        PipelineCreateInfo {
            shaders: vec![
                ShaderModule::compile_glsl(
                    device.clone(),
                    ShaderType::Vertex,
                    include_str!("../shaders/vertex.glsl").to_string(),
                ),
                ShaderModule::compile_glsl(
                    device.clone(),
                    ShaderType::Fragment,
                    include_str!("../shaders/fragment.glsl").to_string(),
                ),
            ],
            depth_write: false,
            depth_test: false,
            front_face: vk::FrontFace::CLOCKWISE,
            cull_mode: vk::CullModeFlags::NONE,
            ..Default::default()
        },
    );

    let buffer_allocator = Arc::new(BufferAllocator::new(&context, &device));
    let buffer = Buffer::new(
        buffer_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsageInfo {
                location: BufferMemoryLocation::PreferHostVisible,
                transfer: BufferTransfert::Destination,
                access: BufferAccess::Sequential,
                usage: BufferUsage::Vertices,
                memory_type: 0,
            },
            alignment: core::mem::align_of::<Vertex2DColor>(),
            data: BufferDataInfo::Slice(&VERTICES),
        },
    );

    command.insert_resource(Render {
        acquire_semaphore: Semaphore::new(device.clone()),
        render_semaphore: Semaphore::new(device.clone()),
        buffer_allocator,
        buffer,
        context,
        device,
        swapchain,
        queues,
        pipeline,
    });
}

// Render the triangle
fn render(render: Res<Render>) {
    let command_pool = CommandPool::new(
        render.device.clone(),
        render.device.queues_info().main_family(),
        vk::CommandPoolCreateFlags::empty(),
    );

    let command = CommandBuffer::new(&command_pool);

    // Acquire the next image from the swapchain. If no image is available,
    // this function wait until an image is available.
    let (image_index, image, iview) = render
        .swapchain
        .acquire_next_image(&render.acquire_semaphore);

    // SAFETY: Most of the following code is safe thank to our encapsulation
    // of the Vulkan API. The only unsafe function call is the `draw` method
    // call because the caller must ensure that the draw call parameters will
    // not cause any out-of-bounds access of any buffer using behind the scenes.
    unsafe {
        command
            .start_recording()
            .pipeline_barrier(PipelineBarrierInfo {
                src_stage_mask: vk::PipelineStageFlags::TOP_OF_PIPE,
                dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                images_barriers: vec![vk::ImageMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_array_layer: 0,
                        base_mip_level: 0,
                        level_count: 1,
                        layer_count: 1,
                    })
                    .image(image)
                    .build()],
            })
            .bind_graphic_pipeline(&render.pipeline)
            .bind_vertex_buffer(&render.buffer)
            .start_rendering(RenderingInfo {
                colors_attachements: vec![vk::RenderingAttachmentInfo::builder()
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .clear_value(vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    })
                    .image_view(iview)
                    .build()],
                render_area: render.swapchain.extent(),
            })
            .draw(DrawInfo {
                vertex_count: 3,
                instance_count: 1,
                first_vertex: 0,
                first_instance: 0,
            })
            .stop_rendering()
            .pipeline_barrier(PipelineBarrierInfo {
                src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                dst_stage_mask: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                images_barriers: vec![vk::ImageMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_access_mask(vk::AccessFlags::empty())
                    .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image)
                    .build()],
            })
            .stop_recording()
            .submit_and_wait(SubmitInfo {
                wait_dst_stage_mask: vec![vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                signal_semaphores: vec![render.render_semaphore.inner()],
                wait_semaphores: vec![render.acquire_semaphore.inner()],
                queue: render.queues.main(),
            });
    };

    // Present the image to the screen
    render.swapchain.present_image(
        render.queues.present(),
        image_index,
        &render.render_semaphore,
    );
}

/// A system that verifies if the application is about to exit. This system returns
/// `true` if the application is about to exit, and `false` otherwise.
pub fn is_exiting(mut event: EventReader<AppExit>) -> bool {
    event.read().next().is_some()
}

/// A system that waits for the device to finish all operations before returning. This
/// is useful when the application is about to exit and we want to make sure that all
/// resources are destroyed before the application closes without destroying a
/// resource that is still in use.
pub fn wait_for_device(render: ResMut<Render>) {
    // Wait for the device to finish all operations before destroying the
    // resources
    unsafe {
        render
            .device
            .logical()
            .device_wait_idle()
            .expect("Failed to wait for device idle")
    };
}
