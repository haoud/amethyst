use amethyst_vulkan::{
    context::VulkanContext,
    device::{VulkanDevice, VulkanQueues},
    pipeline::{NoVertex, Pipeline, PipelineCreateInfo},
    shader::{ShaderModule, ShaderType},
    swapchain::{Surface, VulkanSwapchain},
};
use bevy::{
    prelude::*,
    window::{PrimaryWindow, RawHandleWrapperHolder},
};
use std::sync::Arc;

#[derive(Debug)]
pub struct AmethystRender;

impl Plugin for AmethystRender {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_vulkan_context);
    }
}

#[allow(dead_code)]
#[derive(Debug, Resource)]
pub struct Render {
    pipeline: Pipeline,
    swapchain: VulkanSwapchain,
    queues: VulkanQueues,
    device: Arc<VulkanDevice>,
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

    // SAFETY: Adding plugin to the app should be done in the main thread, so we can
    // safely get the handle in any platform.
    let handle = unsafe { handle.get_handle() };

    let context = Arc::new(VulkanContext::new(&handle));
    let surface = Surface::new(Arc::clone(&context), handle);

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
        context,
        device,
        swapchain,
        queues,
        pipeline,
    });
}
