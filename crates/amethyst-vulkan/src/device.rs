use crate::{
    buffer::allocator::BufferAllocator,
    command::pool::{CommandPool, CommandPoolCreateFlags},
    prelude::{Queue, QueueIndex},
    swapchain::SwapchainSupport,
    Vulkan, REQUIRED_EXTENSIONS,
};
use std::{collections::HashSet, sync::Arc};
use vulkanalia::{prelude::v1_2::*, vk::KhrSurfaceExtension};

// A physical device. This represents a physical device that is
// capable of running the application, and contains information
// about its capabilities.
#[allow(dead_code)]
pub struct PhysicalDevice {
    extensions: Vec<vk::ExtensionProperties>,
    properties: vk::PhysicalDeviceProperties,
    features: vk::PhysicalDeviceFeatures,
    graphics_queue_index: u32,
    compute_queue_index: u32,
    present_queue_index: u32,
    inner: vk::PhysicalDevice,
}

impl PhysicalDevice {
    /// List all compatible physical devices and pick the first one.
    #[must_use]
    pub fn pick_best(vulkan: &Vulkan, info: PhysicalDevicePickInfo) -> Self {
        unsafe {
            vulkan
                .instance()
                .enumerate_physical_devices()
                .expect("Failed to enumerate physical devices")
                .iter()
                .find_map(|&device| PhysicalDevice::suitable(vulkan, device, &info))
                .expect("No suitable physical device found")
        }
    }

    /// Returns true if the physical device is suitable and meets the required
    /// properties and features.
    fn suitable(
        vulkan: &Vulkan,
        physical: vk::PhysicalDevice,
        _info: &PhysicalDevicePickInfo,
    ) -> Option<Self> {
        let (features, properties, extensions, queue_properties) = unsafe {
            let properties = vulkan
                .instance()
                .get_physical_device_properties(physical);
            let features = vulkan
                .instance()
                .get_physical_device_features(physical);
            let extensions = vulkan
                .instance()
                .enumerate_device_extension_properties(physical, None)
                .expect("Failed to enumerate device extensions");
            let queue_properties = vulkan
                .instance()
                .get_physical_device_queue_family_properties(physical);

            (features, properties, extensions, queue_properties)
        };

        // Verify that all required extensions are supported
        let required_extensions_supported = REQUIRED_EXTENSIONS.iter().all(|&required| {
            extensions
                .iter()
                .any(|extension| extension.extension_name == required)
        });

        // Find a graphics queue
        let graphics_queue_index = queue_properties
            .iter()
            .enumerate()
            .find(|(_, &q)| {
                q.queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            })
            .map(|(i, _)| i as u32)?;

        // Find a compute queue
        let compute_queue_index = queue_properties
            .iter()
            .enumerate()
            .find(|(_, &q)| {
                q.queue_flags
                    .contains(vk::QueueFlags::COMPUTE)
            })
            .map(|(i, _)| i as u32)?;

        // Find a present queue
        let present_queue_index = queue_properties
            .iter()
            .enumerate()
            .find(|(i, _)| {
                let surface = vulkan.surface().inner();
                let device = physical;
                let index = *i as u32;

                unsafe {
                    vulkan
                        .instance()
                        .get_physical_device_surface_support_khr(device, index, surface)
                        .expect("Failed to get physical device surface support")
                }
            })
            .map(|(i, _)| i as u32)?;

        // Verify that the physical device supports the swapchain
        let swapchain_support = SwapchainSupport::query(vulkan, physical);
        if !swapchain_support.meet_requirements() {
            return None;
        }

        if required_extensions_supported {
            Some(Self {
                graphics_queue_index,
                compute_queue_index,
                present_queue_index,
                extensions,
                properties,
                features,
                inner: physical,
            })
        } else {
            None
        }
    }

    /// Returns the Vulkan physical device.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::PhysicalDevice {
        self.inner
    }
}

/// Information used to pick a physical device.
pub struct PhysicalDevicePickInfo {}

impl Default for PhysicalDevicePickInfo {
    fn default() -> Self {
        Self {}
    }
}

/// A logical device. This is used to communicate with a physical device
/// and contains information about the queues that can be used from the
/// physical device.
#[allow(dead_code)]
pub struct LogicalDevice {
    physical: PhysicalDevice,
    inner: Device,
}

impl LogicalDevice {
    #[must_use]
    pub fn new(vulkan: &Vulkan, physical: PhysicalDevice, _: LogicalDeviceCreateInfo) -> Self {
        // The device queues creation does not support duplicates, so we need to
        // create a set of unique indices and then create the queues from it
        let queues_indices_set = HashSet::from([
            physical.graphics_queue_index,
            physical.compute_queue_index,
            physical.present_queue_index,
        ]);

        let queue_priorities = &[1.0];
        let queues_info = queues_indices_set
            .iter()
            .map(|i| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_priorities(queue_priorities)
                    .queue_family_index(*i)
                    .build()
            })
            .collect::<Vec<_>>();

        // Enable extensions required by the engine
        let extensions = REQUIRED_EXTENSIONS
            .iter()
            .map(|extension| extension.as_ptr())
            .collect::<Vec<_>>();

        // The engine currently need the dynamic rendering feature to work properly.
        // However, this feature is supported only by Vulkan 1.3 and above. So we need
        // to enable it manually.
        let mut vulkan_1_3_features = vk::PhysicalDeviceVulkan13Features::builder()
            .dynamic_rendering(true)
            .build();

        let features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)
            .build();

        // Prepare the creation info for the logical device and create it
        let creation_info = vk::DeviceCreateInfo::builder()
            .enabled_extension_names(&extensions)
            .queue_create_infos(&queues_info)
            .enabled_features(&features)
            .push_next(&mut vulkan_1_3_features);

        let inner = unsafe {
            vulkan
                .instance()
                .create_device(physical.inner(), &creation_info, None)
                .expect("Failed to create logical device")
        };

        Self { physical, inner }
    }

    /// Pick the best physical device and create a logical device from it.
    /// This is a shortcut for calling `PhysicalDevice::pick_best` followed
    /// by creating a logical device from the result.
    #[must_use]
    pub fn pick_best(vulkan: &Vulkan, _: LogicalDeviceCreateInfo) -> Self {
        let physical = PhysicalDevice::pick_best(
            &vulkan,
            PhysicalDevicePickInfo {
                ..Default::default()
            },
        );

        Self::new(vulkan, physical, Default::default())
    }

    /// Wait for the device to be idle before returning.
    pub fn wait_idle(&self) {
        unsafe {
            self.inner
                .device_wait_idle()
                .expect("Failed to wait for device idle");
        }
    }

    /// Returns the physical device.
    #[must_use]
    pub fn physical(&self) -> &PhysicalDevice {
        &self.physical
    }

    /// Returns the Vulkan logical device.
    #[must_use]
    pub(crate) fn inner(&self) -> &Device {
        &self.inner
    }
}

impl Drop for LogicalDevice {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_device(None);
        }
    }
}

/// Information used to create a logical device.
pub struct LogicalDeviceCreateInfo {}

impl Default for LogicalDeviceCreateInfo {
    fn default() -> Self {
        Self {}
    }
}

/// A render device. A render device is a logical device with additional information
/// and objects used to render the scene: It contains a buffer allocator, a command
/// pool, and different queues.
pub struct RenderDevice {
    buffer_allocator: BufferAllocator,
    commands_pool: CommandPool,
    compute_queue: Queue,
    graphic_queue: Queue,
    present_queue: Queue,

    logical: Arc<LogicalDevice>,
    vulkan: Arc<Vulkan>,
}

impl RenderDevice {
    /// Create a new render device by picking the best physical device and creating
    /// a logical device from it.
    #[must_use]
    pub fn pick_best(vulkan: Arc<Vulkan>, _: RenderDevicePickInfo) -> Self {
        let physical = PhysicalDevice::pick_best(
            &vulkan,
            PhysicalDevicePickInfo {
                ..Default::default()
            },
        );

        let logical = Arc::new(LogicalDevice::new(
            &vulkan,
            physical,
            LogicalDeviceCreateInfo {
                ..Default::default()
            },
        ));

        let graphic_queue = unsafe {
            let queue = logical
                .inner()
                .get_device_queue(logical.physical().graphics_queue_index, 0);
            Queue::new(
                queue,
                QueueIndex::new(logical.physical().graphics_queue_index),
            )
        };

        let compute_queue = unsafe {
            let queue = logical
                .inner()
                .get_device_queue(logical.physical().compute_queue_index, 0);
            Queue::new(
                queue,
                QueueIndex::new(logical.physical().compute_queue_index),
            )
        };

        let present_queue = unsafe {
            let queue = logical
                .inner()
                .get_device_queue(logical.physical().present_queue_index, 0);
            Queue::new(
                queue,
                QueueIndex::new(logical.physical().present_queue_index),
            )
        };

        let commands_pool = CommandPool::new(
            Arc::clone(&logical),
            &graphic_queue,
            CommandPoolCreateFlags::empty(),
        );

        let buffer_allocator = BufferAllocator::new(&vulkan, &logical);

        Self {
            buffer_allocator,
            commands_pool,
            compute_queue,
            graphic_queue,
            present_queue,
            logical,
            vulkan,
        }
    }

    /// Returns the buffer allocator.
    #[must_use]
    pub fn buffer_allocator(&self) -> &BufferAllocator {
        &self.buffer_allocator
    }

    /// Returns the command pool used to allocate command buffers.
    #[must_use]
    pub fn commands_pool(&self) -> &CommandPool {
        &self.commands_pool
    }

    /// Returns the compute queue selected
    #[must_use]
    pub fn compute_queue(&self) -> &Queue {
        &self.compute_queue
    }

    /// Returns the graphic queue
    #[must_use]
    pub fn graphic_queue(&self) -> &Queue {
        &self.graphic_queue
    }

    /// Returns the present queue
    #[must_use]
    pub fn present_queue(&self) -> &Queue {
        &self.present_queue
    }

    /// Returns the logical device.
    #[must_use]
    pub fn logical(&self) -> &LogicalDevice {
        &self.logical
    }

    /// Returns the physical device.
    #[must_use]
    pub fn physical(&self) -> &PhysicalDevice {
        &self.logical.physical()
    }

    /// Returns the Vulkan instance.
    #[must_use]
    pub fn vulkan(&self) -> &Arc<Vulkan> {
        &self.vulkan
    }
}

/// Information used to pick a render device.
pub struct RenderDevicePickInfo {}

impl Default for RenderDevicePickInfo {
    fn default() -> Self {
        Self {}
    }
}
