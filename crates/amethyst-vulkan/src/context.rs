use bevy::{prelude::*, window::ThreadLockedRawWindowHandleWrapper};
use std::{borrow::Cow, collections::HashSet, ffi::CStr};
use vk::ExtDebugUtilsExtension;
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY},
    prelude::v1_3::*,
};

/// The name of the engine. This is used to identify the engine when creating the Vulkan instance,
/// and should be unique to the engine as it is used by the driver to optimize for the engine if
/// it has specific optimizations. It will probably never be the case that Amethyst will have
/// specific optimizations built into drivers, but we never know :)
pub static ENGINE_NAME: &[u8] = b"Amethyst\0";

/// The name of the application. This is used to identify the application when creating the Vulkan
/// instance, for the same reasons as the engine name ([`ENGINE_NAME`]).
pub static APPLICATION_NAME: &[u8] = b"Amethyst application\0";

/// The name of the validation layer. This is used to enable the validation layer when creating the
/// Vulkan instance. This is only used in debug builds, and is not used in release builds.
pub static VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

/// Whether to enable validation layers. This is only enabled in debug builds, and is disabled in
/// release builds.
pub const ENABLE_VALIDATION: bool = cfg!(debug_assertions);

#[allow(dead_code)]
#[derive(Debug, Resource)]
pub struct VulkanContext {
    /// The entry point to the Vulkan API
    entry: vulkanalia::Entry,

    /// The Vulkan instance
    instance: vulkanalia::Instance,

    // The debug messenger. This is only created if validation layers are enabled.
    messenger: Option<vk::DebugUtilsMessengerEXT>,
}

impl VulkanContext {
    #[must_use]
    pub fn new(handle: &ThreadLockedRawWindowHandleWrapper) -> Self {
        let entry = unsafe {
            let loader = LibloadingLoader::new(LIBRARY).expect("Failed to load Vulkan loader");
            Entry::new(loader).expect("Failed to load Vulkan entry point")
        };

        // Enumerate the available instance layers and store them in a set
        // for easy lookup and without duplicates.
        let available_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .expect("Failed to enumerate instance layers")
                .iter()
                .map(|l| l.layer_name)
                .collect::<HashSet<_>>()
        };

        // If the validation layer is available and validation is enabled, add the validation
        // layer to the list of layers to enable. If at least one condition is not met, disable
        // validation by not adding any layers.
        let layers = if !available_layers.is_empty() && ENABLE_VALIDATION {
            if available_layers.contains(&VALIDATION_LAYER) {
                vec![VALIDATION_LAYER.as_ptr()]
            } else {
                debug!("Validation layer not available, disabling validation");
                vec![]
            }
        } else {
            vec![]
        };

        // Create the application info with the application and engine names,
        // versions, and the Vulkan API version. This does not really matter
        // except for the Vulkan API version, which should be set to the version
        // of Vulkan that the application is targeting.
        let application_info = vk::ApplicationInfo::builder()
            .application_version(vk::make_version(0, 1, 0))
            .engine_version(vk::make_version(0, 1, 0))
            .api_version(vk::make_version(1, 3, 0))
            .application_name(APPLICATION_NAME)
            .engine_name(ENGINE_NAME);

        let mut required_instance_extensions =
            vulkanalia::window::get_required_instance_extensions(&handle)
                .iter()
                .map(|name| name.as_ptr())
                .collect::<Vec<_>>();

        // If validation is enabled, add the validation layer to the list of required instance
        // extensions to enable the validation layer.
        if !layers.is_empty() {
            required_instance_extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
        }

        // Create the Vulkan instance with the required extensions, layers, and application
        // info previously created.
        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_extension_names(&required_instance_extensions)
            .enabled_layer_names(&layers)
            .build();

        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .expect("Failed to create Vulkan instance")
        };

        // Create the debug messenger if validation is enabled.
        let mut messenger = None;
        if ENABLE_VALIDATION && !layers.is_empty() {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                .user_callback(Some(vulkan_debug_callback));

            messenger = unsafe {
                instance
                    .create_debug_utils_messenger_ext(&debug_info, None)
                    .ok()
            };
        }

        Self {
            entry,
            instance,
            messenger,
        }
    }

    /// Returns the Vulkan instance object.
    #[must_use]
    pub const fn instance(&self) -> &Instance {
        &self.instance
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            if let Some(messenger) = self.messenger {
                self.instance
                    .destroy_debug_utils_messenger_ext(messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

/// The Vulkan debug callback. This is used to print validation layer messages. The output
/// can be controlled by the user with the `RUST_LOG` environment variable or by properly
/// configuring the logger.
extern "system" fn vulkan_debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    kind: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = if data.message.is_null() {
        Cow::Borrowed("No message provided")
    } else {
        unsafe { CStr::from_ptr(data.message) }.to_string_lossy()
    };

    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("[{:?}] {}", kind, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("[{:?}] {}", kind, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("[{:?}] {}", kind, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            log::debug!("[{:?}] {}", kind, message);
        }
        _ => {
            log::trace!("[{:?}] {}", kind, message);
        }
    }
    vk::FALSE
}
