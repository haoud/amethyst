use amethyst_vulkan::{
    command::{CommandBuffer, CommandPool},
    context::VulkanContext,
    device::{VulkanDevice, VulkanQueues},
    pipeline::{NoVertex, Pipeline, PipelineCreateInfo},
    semaphore::Semaphore,
    shader::{ShaderModule, ShaderType},
    swapchain::{Surface, VulkanSwapchain},
};
use bevy::{
    prelude::*,
    window::{PrimaryWindow, RawHandleWrapperHolder},
};
use std::sync::Arc;
use vulkanalia::prelude::v1_3::*;

#[derive(Debug)]
pub struct AmethystRender;

impl Plugin for AmethystRender {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_vulkan_context);
        app.add_systems(Update, render);
    }
}

/// # Important
/// The order of fields in the struct is important to ensure that the resources are dropped in the
/// right order. The fields are dropped in the order they are defined in the struct, and some fields
/// must be destroyed before others. Please keep this in mind when adding new fields to the struct.
#[derive(Debug, Resource)]
pub struct Render {
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
    let surface = Surface::new(Arc::clone(&context), handle);

    // Create the device, swapchain, and queues objects
    let device = Arc::new(VulkanDevice::pick_best(&context, &surface));
    let swapchain = VulkanSwapchain::new(Arc::clone(&context), Arc::clone(&device), surface);
    let queues = VulkanQueues::fetch(&device);

    // Create a pipeline object that does not require vertex data and
    // use a simple vertex and fragment shader. Since we are trying to
    // render a simple triangle, we don't need to pass any vertex data
    // to the vertex shader (hence the `NoVertex` type) and we also don't
    // need to write to the depth buffer.
    let pipeline = Pipeline::new::<NoVertex>(
        Arc::clone(&device),
        &swapchain,
        PipelineCreateInfo {
            shaders: vec![
                ShaderModule::compile_glsl(
                    Arc::clone(&device),
                    ShaderType::Vertex,
                    include_str!("../shaders/vertex.glsl").to_string(),
                ),
                ShaderModule::compile_glsl(
                    Arc::clone(&device),
                    ShaderType::Fragment,
                    include_str!("../shaders/fragment.glsl").to_string(),
                ),
            ],

            depth_write: false,
            depth_test: false,
            ..Default::default()
        },
    );

    command.insert_resource(Render {
        acquire_semaphore: Semaphore::new(Arc::clone(&device)),
        render_semaphore: Semaphore::new(Arc::clone(&device)),
        context,
        device,
        swapchain,
        queues,
        pipeline,
    });
}

fn render(render: Res<Render>) {
    let command_pool = CommandPool::new(
        Arc::clone(&render.device),
        render.device.queues_info().main_family(),
        vk::CommandPoolCreateFlags::empty(),
    );

    let command = CommandBuffer::new(&command_pool);

    let image_index = render
        .swapchain
        .acquire_next_image_index(&render.acquire_semaphore);
    let image = render.swapchain.images()[image_index as usize];
    let iview = render.swapchain.image_views()[image_index as usize];

    // Begin the command buffer recording
    let begin_info = vk::CommandBufferBeginInfo::builder()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
        .build();
    unsafe {
        render
            .device
            .logical()
            .begin_command_buffer(command.inner(), &begin_info)
            .expect("Failed to begin command buffer");
    }

    // Transition the swapchain image to a layout that is suitable for rendering
    let image_barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_array_layer: 0,
            base_mip_level: 0,
            level_count: 1,
            layer_count: 1,
        });
    let buffers_barriers: [vk::BufferMemoryBarrier; 0] = [];
    let memories_barriers: [vk::MemoryBarrier; 0] = [];

    unsafe {
        render.device.logical().cmd_pipeline_barrier(
            command.inner(),
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &memories_barriers,
            &buffers_barriers,
            &[image_barrier],
        );
    }

    // Start the rendering using the dynamic rendering feature
    unsafe {
        let color_attachment = [vk::RenderingAttachmentInfo::builder()
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .store_op(vk::AttachmentStoreOp::STORE)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            })
            .image_view(iview)];

        let render_area = vk::Rect2D::builder()
            .extent(render.swapchain.extent())
            .build();

        let rendering_info = vk::RenderingInfo::builder()
            .color_attachments(&color_attachment)
            .render_area(render_area)
            .layer_count(1);

        render
            .device
            .logical()
            .cmd_begin_rendering(command.inner(), &rendering_info);
    }

    // Bind the pipeline object to the command buffer
    unsafe {
        render.device.logical().cmd_bind_pipeline(
            command.inner(),
            vk::PipelineBindPoint::GRAPHICS,
            render.pipeline.inner(),
        );
    }

    // Draw the triangle
    unsafe {
        render
            .device
            .logical()
            .cmd_draw(command.inner(), 3, 1, 0, 0);
    }

    // End the rendering
    unsafe {
        render.device.logical().cmd_end_rendering(command.inner());
    }

    // Transition the swapchain image to a layout that is suitable for presenting
    // the image to the screen
    let image_barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::empty())
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    unsafe {
        render.device.logical().cmd_pipeline_barrier(
            command.inner(),
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &memories_barriers,
            &buffers_barriers,
            &[image_barrier],
        );
    }

    // End the command buffer recording
    unsafe {
        render
            .device
            .logical()
            .end_command_buffer(command.inner())
            .expect("Failed to end command buffer");
    }

    // Submit the command buffer to the graphic queue
    let command_buffers = [command.inner()];
    let wait_semaphores = [render.acquire_semaphore.inner()];
    let signal_semaphores = [render.render_semaphore.inner()];
    let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
    let submit_info = vk::SubmitInfo::builder()
        .signal_semaphores(&signal_semaphores)
        .wait_semaphores(&wait_semaphores)
        .command_buffers(&command_buffers)
        .wait_dst_stage_mask(&wait_dst_stage_mask);

    unsafe {
        render
            .device
            .logical()
            .queue_submit(render.queues.main(), &[submit_info], vk::Fence::null())
            .expect("Failed to submit command buffer to graphics queue");
    }

    // Present the image to the screen
    render.swapchain.present_image(
        render.queues.present(),
        image_index,
        &render.render_semaphore,
    );

    // Wait for the graphic queue to finish rendering
    unsafe {
        render
            .device
            .logical()
            .queue_wait_idle(render.queues.main())
            .expect("Failed to wait for graphic queue to finish rendering");
    }
}
