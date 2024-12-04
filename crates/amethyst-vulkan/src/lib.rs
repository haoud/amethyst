use bevy::{
    prelude::*,
    window::{PrimaryWindow, RawHandleWrapperHolder},
};
use context::VulkanContext;
use device::{VulkanDevice, VulkanQueues};

pub mod context;
pub mod device;

#[derive(Debug)]
pub struct VulkanPlugin;

impl Plugin for VulkanPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_vulkan_context);
    }
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

    command.insert_resource(device);
    command.insert_resource(context);
    command.insert_resource(queues);
}
