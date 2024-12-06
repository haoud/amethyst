use amethyst_vulkan::{
    context::VulkanContext,
    device::{VulkanDevice, VulkanQueues},
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

#[derive(Debug, Resource)]
pub struct Render {
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

    let context = VulkanContext::new(&handle);
    let device = VulkanDevice::pick_best(&context);
    let queues = VulkanQueues::fetch(&device);

    command.insert_resource(Render {
        context: Arc::new(context),
        device: Arc::new(device),
        queues,
    });
}
