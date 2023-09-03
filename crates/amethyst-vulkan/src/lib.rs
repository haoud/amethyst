#![allow(dead_code)]

use amethyst_core::prelude::Engine;
use amethyst_window::Window;
use prelude::Surface;
use std::{
    collections::HashSet,
    ffi::{c_void, CStr, CString},
    sync::Arc,
};
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY},
    prelude::v1_2::*,
    vk::ExtDebugUtilsExtension,
};

static REQUIRED_EXTENSIONS: [vk::ExtensionName; 1] = [vk::KHR_SWAPCHAIN_EXTENSION.name];
static DEBUG_UTILS_EXTENSION: vk::ExtensionName = vk::EXT_DEBUG_UTILS_EXTENSION.name;
static VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

pub mod buffer;
pub mod command;
pub mod descriptor;
pub mod device;
pub mod image;
pub mod pipeline;
pub mod prelude;
pub mod queue;
pub mod shader;
pub mod surface;
pub mod swapchain;
pub mod sync;

/// A vulkan object. It contains some global state, such as the instance, that
/// is needed to create other vulkan objects.
///
/// # Developper notes
/// The order of destruction is important here, as some fields must be destroyed
/// before others. Please refer to the Vulkan documentation for more information
/// and be careful when changing the order of the fields.
#[allow(dead_code)]
pub struct Vulkan {
    surface: Surface,
    messenger: Option<vk::DebugUtilsMessengerEXT>,
    context: Context,
    entry: Entry,
}

impl Vulkan {
    #[must_use]
    pub fn new(_engine: &mut Engine, mut info: VulkanInfo) -> Self {
        let window = info
            .window
            .expect("Offscreen rendering is not yet supported");

        let entry = unsafe {
            Entry::new(LibloadingLoader::new(LIBRARY).expect("Failed to load Vulkan library"))
                .expect("Failed to create Vulkan instance")
        };

        let application_name = CString::new(info.application_name).unwrap();
        let engine_name = CString::new("Amethyst").unwrap();

        let application_info = vk::ApplicationInfo::builder()
            .application_name(&application_name.as_bytes())
            .application_version(vk::make_version(
                info.application_version.0,
                info.application_version.1,
                info.application_version.2,
            ))
            .engine_version(vk::make_version(0, 1, 0))
            .engine_name(&engine_name.as_bytes())
            .api_version(vk::make_version(1, 3, 0));

        // Enumerate the available layers
        let available_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .expect("Failed to enumerate instance layers")
                .iter()
                .map(|l| l.layer_name)
                .collect::<HashSet<_>>()
        };

        // If no validation layer is available, we disable validation and simply we continue.
        if info.enable_validation && !available_layers.contains(&VALIDATION_LAYER) {
            log::warn!("Validation layer not available, disabling validation");
            info.enable_validation = false;
        }

        let mut layers = Vec::new();
        if info.enable_validation {
            layers.push(VALIDATION_LAYER.as_ptr());
        }

        // Because we require to have a window, we must also require the swapchain extension
        // and the surface extension (operating system dependent) to be available.
        let mut extensions = vulkanalia::window::get_required_instance_extensions(window.inner())
            .iter()
            .map(|s| s.as_ptr())
            .collect::<Vec<_>>();

        // If validation is enabled, we also need the debug utils extension
        if info.enable_validation {
            extensions.push(DEBUG_UTILS_EXTENSION.as_ptr());
        }

        // Prepare the instance creation
        let mut instance_info = vk::InstanceCreateInfo::builder()
            .enabled_extension_names(&extensions)
            .application_info(&application_info)
            .enabled_layer_names(&layers)
            .flags(vk::InstanceCreateFlags::empty());

        let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .user_callback(Some(vulkan_debug_callback));

        if info.enable_validation && info.enable_instance_validation {
            instance_info = instance_info.push_next(&mut debug_info);
        }

        // Create the instance
        let instance = unsafe {
            entry
                .create_instance(&instance_info, None)
                .expect("Failed to create Vulkan instance")
        };

        let messenger = if info.enable_validation {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                .user_callback(Some(vulkan_debug_callback));

            Some(unsafe {
                instance
                    .create_debug_utils_messenger_ext(&debug_info, None)
                    .expect("Failed to create Vulkan debug messenger")
            })
        } else {
            None
        };

        let context = Context {
            inner: Arc::new(instance),
        };

        let surface = Surface::from_window(Arc::clone(&context.inner), &window);

        Vulkan {
            messenger,
            context,
            surface,
            entry,
        }
    }

    /// Returns the vulkan instance
    #[must_use]
    pub(crate) fn instance(&self) -> &Arc<Instance> {
        &self.context.inner
    }

    /// Returns the surface where the engine is rendering to.
    #[must_use]
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe {
            if let Some(messenger) = self.messenger {
                self.context
                    .inner
                    .destroy_debug_utils_messenger_ext(messenger, None);
            }
        }
    }
}

struct Context {
    inner: Arc<Instance>,
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_instance(None);
        }
    }
}

pub struct VulkanInfo<'a> {
    /// The application name. This is used by the Vulkan driver for identification purposes.
    pub application_name: &'a str,

    /// The application version. This is used by the Vulkan driver for identification purposes.
    /// The version is split into three parts: major, minor and patch.
    pub application_version: (u32, u32, u32),

    /// Whether to enable validation layers or not. If validation layers are enabled, the Vulkan
    /// driver will perform additional checks and print warnings if the application is doing
    /// something wrong. This is useful during development, but it has a performance cost.
    pub enable_validation: bool,

    /// Enable validation layers for the instance creation. This is useful if you want to
    /// validate the instance creation itself.
    /// This flag requires `enable_validation` to be true, otherwise it will be ignored.
    pub enable_instance_validation: bool,

    /// The window to render to. For now, this parameter is mandatory, but in the future, it will
    /// be possible to render to an offscreen buffer.
    pub window: Option<&'a Window>,
}

impl Default for VulkanInfo<'_> {
    fn default() -> Self {
        VulkanInfo {
            application_name: "Amethyst application",
            application_version: (0, 0, 1),
            enable_instance_validation: false,
            enable_validation: false,
            window: None,
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
    _: *mut c_void,
) -> vk::Bool32 {
    let message = unsafe { CStr::from_ptr((*data).message).to_string_lossy() };
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
