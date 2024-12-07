use crate::{
    context::{VulkanContext, ENABLE_VALIDATION, VALIDATION_LAYER},
    swapchain::Surface,
};
use bevy::prelude::*;
use std::collections::HashSet;
use vk::KhrSurfaceExtension;
use vulkanalia::prelude::v1_3::*;

/// The device extensions required by Amethyst.
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

/// The Vulkan device. This contains the physical device chosen by Amethyst, the logical device
/// created from the physical device, and information about the queues of the device.
#[derive(Debug, Resource)]
pub struct VulkanDevice {
    /// The physical device chosen by Amethyst
    physical: vk::PhysicalDevice,

    /// The logical device created from the physical device. This is an abstraction
    /// over the physical device, and is used to interact with the physical device.
    logical: Device,

    /// Information about the queues of the device. This contains the main queue family
    /// that supports graphics, compute, and transfer operations, and optional async
    /// transfer and async compute queue families that support transfer and compute
    /// operations, respectively.
    queues_info: DeviceQueueInfo,
}

impl VulkanDevice {
    /// Choose the best physical device and create a logical device from it.
    #[must_use]
    pub fn pick_best(context: &VulkanContext, surface: &Surface) -> Self {
        let physical = unsafe {
            let mut devices = context
                .instance()
                .enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
                .into_iter()
                .map(|physical| {
                    let properties = context.instance().get_physical_device_properties(physical);
                    let features = context.instance().get_physical_device_features(physical);
                    (physical, properties, features)
                })
                .collect::<Vec<_>>();

            // Sort the physical devices by type, with discrete GPUs first, then integrated GPUs,
            // and finally virtual GPUs. This is done to prioritize discrete GPUs over integrated
            // GPUs, as discrete GPUs are generally more powerful and have better performance.
            devices.sort_by_key(|(_, properties, _)| match properties.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                _ => 3,
            });

            // Find the first physical device that has all the required features and
            // properties. Since the physical devices are sorted by its potential
            // performance, the first physical device that meets the requirements should
            // be the best physical device for the application.
            devices
                .into_iter()
                .find(|(device, properties, features)| {
                    Self::suitable_device(context, device, properties, features)
                })
                .expect("No suitable physical device found")
                .0
        };

        // Retrieve the queues from the logical device. Try to get separate
        // queues for performance reasons, but fall back to a single queue
        // if separate queues are not available.
        let queues_info = DeviceQueueInfo::new(context, physical, surface);

        // Add the main queue family to the list of queues to create. Since Amethyst
        // only use one queue per queue family, the queue priority does not really
        // matter here and can be set to 1.0.
        let queue_priorities = [1.0];

        // Create a set of queue families that should be created. This avoid trying to create
        // request the same queue family multiple times, which is not allowed by Vulkan.
        let queues_family_set = HashSet::from([
            queues_info.main_family(),
            queues_info.present_family(),
            queues_info
                .async_compute_family()
                .unwrap_or(queues_info.main_family()),
            queues_info
                .async_transfer_family()
                .unwrap_or(queues_info.main_family()),
        ]);

        // Create the queue create info for each queue family needed.
        let queues_create_info = queues_family_set
            .into_iter()
            .map(|family| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(family)
                    .queue_priorities(&queue_priorities)
                    .build()
            })
            .collect::<Vec<_>>();

        // Add the validation layer to the list of layers to enable if validation is enabled.
        let layers_names = if ENABLE_VALIDATION {
            vec![VALIDATION_LAYER.as_ptr()]
        } else {
            vec![]
        };

        // The list of extensions to enable for the logical device. This should include the
        // swapchain extension, as it is required for rendering to the screen. Then, create the
        // device create info with the queues, extensions, layers, and features.
        let extensions = DEVICE_EXTENSIONS
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<_>>();
        let features = vk::PhysicalDeviceFeatures::builder().sampler_anisotropy(true);
        let mut feature_1_3 = vk::PhysicalDeviceVulkan13Features::builder()
            .dynamic_rendering(true)
            .synchronization2(true);
        let device_create_info = vk::DeviceCreateInfo::builder()
            .enabled_extension_names(&extensions)
            .enabled_layer_names(&layers_names)
            .queue_create_infos(&queues_create_info)
            .enabled_features(&features)
            .push_next(&mut feature_1_3);

        // Create the logical device from the physical device,
        // queue info, and device features.
        let logical = unsafe {
            context
                .instance()
                .create_device(physical, &device_create_info, None)
                .expect("Failed to create logical device")
        };

        Self {
            physical,
            logical,
            queues_info,
        }
    }

    /// Verify if the physical device is suitable for the application. This checks if the physical
    /// device supports all the required features, capabilities, and extensions needed by Amethyst.
    pub fn suitable_device(
        context: &VulkanContext,
        device: &vk::PhysicalDevice,
        _properties: &vk::PhysicalDeviceProperties,
        _features: &vk::PhysicalDeviceFeatures,
    ) -> bool {
        // Get all the extensions supported by the physical device.
        let extensions = unsafe {
            context
                .instance()
                .enumerate_device_extension_properties(*device, None)
                .expect("Failed to enumerate device extensions")
                .iter()
                .map(|e| e.extension_name)
                .collect::<HashSet<_>>()
        };

        // Check if the physical device supports all the required extensions.
        if !DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
            return false;
        }

        // TODO: Verify that extensions like dynamic rendering and synchronization2 are supported
        true
    }

    /// Returns the vulkan physical device object.
    #[must_use]
    pub const fn physical(&self) -> vk::PhysicalDevice {
        self.physical
    }

    /// Returns the vulkan logical device object.
    #[must_use]
    pub const fn logical(&self) -> &Device {
        &self.logical
    }

    /// Returns the device queues information.
    #[must_use]
    pub const fn queues_info(&self) -> &DeviceQueueInfo {
        &self.queues_info
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            self.logical.destroy_device(None);
        }
    }
}

/// The device queues. This contains the main queue family that supports graphics, compute, and
/// transfer operations, and optional async transfer and async compute queue families that support
/// transfer and compute operations, respectively.
#[derive(Debug)]
pub struct DeviceQueueInfo {
    main: u32,
    present: u32,
    async_transfer: Option<u32>,
    async_compute: Option<u32>,
}

impl DeviceQueueInfo {
    /// Create a new set of device queues from the physical device. This will find the main queue
    /// that supports graphics, compute, and transfer operations, and try to find async transfer
    /// and async compute queues that support transfer and compute operations, respectively.
    #[must_use]
    pub fn new(context: &VulkanContext, device: vk::PhysicalDevice, surface: &Surface) -> Self {
        let families = unsafe {
            context
                .instance()
                .get_physical_device_queue_family_properties(device)
                .iter()
                .enumerate()
                .map(|(index, properties)| {
                    let flags = properties.queue_flags;
                    (index as u32, flags)
                })
                .collect::<Vec<_>>()
        };

        // Vulkan standard requires that at least one queue family supports graphics, compute,
        // and transfer operations. This is the main queue family, and is used for most operations.
        let main = families
            .iter()
            .find(|(_, flags)| {
                flags.contains(vk::QueueFlags::GRAPHICS)
                    && flags.contains(vk::QueueFlags::COMPUTE)
                    && flags.contains(vk::QueueFlags::TRANSFER)
            })
            .map(|(index, _)| index)
            .expect("No main queue family found");

        // Find a queue family that supports presenting to the surface. This is used for
        // presenting the rendered images to the screen. It may be the same as the main queue,
        // but this does not really matter for most applications.
        let present = families
            .iter()
            .find(|(index, _)| unsafe {
                context
                    .instance()
                    .get_physical_device_surface_support_khr(device, *index, surface.inner())
                    .expect("Failed to get surface support")
            })
            .map(|(index, _)| *index)
            .expect("No present queue family found");

        // Try to find a queue family that supports transfer operations, but is not the main queue
        // family. This is used for async transfer operations alongside graphics and compute
        // operations, enabling parallelism between transfer and graphics/compute operations.
        let async_transfer = families
            .iter()
            .find(|(index, flags)| flags.contains(vk::QueueFlags::TRANSFER) && index != main)
            .map(|(index, _)| index)
            .copied();

        // Try to find a queue family that supports compute operations, but is not the main queue
        // family. This is used for async compute operations alongside graphics and transfer
        // operations, enabling parallelism between compute and graphics/transfer operations.
        let async_compute = families
            .iter()
            .find(|(index, flags)| flags.contains(vk::QueueFlags::COMPUTE) && index != main)
            .map(|(index, _)| index)
            .copied();

        Self {
            main: *main,
            present,
            async_transfer,
            async_compute,
        }
    }

    /// Verify if the device has separate async transfer and async compute capabilities.
    #[must_use]
    pub fn has_separate_async_compute_transfer(&self) -> bool {
        self.async_transfer != self.async_compute
    }

    /// Verify if the device has async transfer capabilities.
    #[must_use]
    pub const fn has_async_transfer(&self) -> bool {
        self.async_transfer.is_some()
    }

    /// Verify if the device has async compute capabilities.
    #[must_use]
    pub const fn has_async_compute(&self) -> bool {
        self.async_compute.is_some()
    }

    /// Returns the async transfer queue family index, which supports transfer operations
    /// but is not the main queue family. This is used for async transfer operations
    /// alongside graphics and compute operations.
    ///
    /// ## Returns
    /// The async transfer queue family index, or `None` if the device does not support async
    /// transfer operations.
    #[must_use]
    pub const fn async_transfer_family(&self) -> Option<u32> {
        self.async_transfer
    }

    /// Returns the async compute queue family index, which supports compute operations
    /// but is not the main queue family. This is used for async compute operations
    /// alongside graphics and transfer operations.
    ///
    /// ## Returns
    /// The async compute queue family index, or `None` if the device does not support async
    /// compute operations.
    #[must_use]
    pub const fn async_compute_family(&self) -> Option<u32> {
        self.async_compute
    }

    /// Returns the present queue family index, which supports presenting to the surface.
    /// This is used for presenting the rendered images to the screen.
    #[must_use]
    pub const fn present_family(&self) -> u32 {
        self.present
    }

    /// Returns the main queue family index, which supports graphics, compute, and
    /// transfer operations. This is the main queue family used for most operations.
    #[must_use]
    pub const fn main_family(&self) -> u32 {
        self.main
    }
}

/// The device queues. This contains the main queue that supports graphics, compute, and transfer
/// operations, and optional async transfer and async compute queues that support transfer and
/// compute operations, respectively, allowing for parallelism between different operations and
/// better performance.
#[derive(Debug, Resource)]
pub struct VulkanQueues {
    main: vk::Queue,
    present: vk::Queue,
    async_transfer: Option<vk::Queue>,
    async_compute: Option<vk::Queue>,
}

impl VulkanQueues {
    /// Fetch the device queues from the logical device. This will get the main queue that supports
    /// graphics, compute, and transfer operations, and try to get async transfer and async compute
    /// queues that support transfer and compute operations, respectively.
    ///
    /// ## Note
    /// This function will only get the first queue from each queue family. If the device has
    /// multiple queues in a queue family, this function will only get the first queue in the
    /// queue family, and will never use the other queues in the queue family.
    #[must_use]
    pub fn fetch(device: &VulkanDevice) -> Self {
        let main = unsafe {
            device
                .logical()
                .get_device_queue(device.queues_info().main_family(), 0)
        };

        let present = unsafe {
            device
                .logical()
                .get_device_queue(device.queues_info().present_family(), 0)
        };

        let async_transfer = device
            .queues_info()
            .async_transfer_family()
            .map(|family| unsafe { device.logical().get_device_queue(family, 0) });

        let async_compute = device
            .queues_info()
            .async_compute_family()
            .map(|family| unsafe { device.logical().get_device_queue(family, 0) });

        Self {
            main,
            present,
            async_transfer,
            async_compute,
        }
    }

    /// Returns the main queue that supports graphics, compute, and transfer operations.
    /// This is the main queue used for most operations.
    #[must_use]
    pub const fn main(&self) -> vk::Queue {
        self.main
    }

    /// Returns the present queue that supports presenting to the surface.
    #[must_use]
    pub const fn present(&self) -> vk::Queue {
        self.present
    }

    /// Returns the async transfer queue that supports transfer operations but is not the main queue.
    /// This is used for async transfer operations alongside graphics and compute operations.
    ///
    /// ## Returns
    /// The async transfer queue, or `None` if the device does not support async transfer operations.
    /// This is the main queue used for most operations.
    #[must_use]
    pub const fn async_transfer(&self) -> Option<vk::Queue> {
        self.async_transfer
    }

    /// Returns the async compute queue that supports compute operations but is not the main queue.
    /// This is used for async compute operations alongside graphics and transfer operations.
    ///
    /// ## Returns
    /// The async compute queue, or `None` if the device does not support async compute operations.
    /// This is the main queue used for most operations.
    #[must_use]
    pub const fn async_compute(&self) -> Option<vk::Queue> {
        self.async_compute
    }
}
