use crate::{device::LogicalDevice, Vulkan};
use vk_mem_vulkanalia::{Allocator, AllocatorCreateInfo};

/// A buffer allocator. It uses the Vulkan Memory Allocator library to allocate
/// buffers.
pub struct BufferAllocator {
    inner: Allocator,
}

impl BufferAllocator {
    /// Create a new buffer allocator for a logical device.
    #[must_use]
    pub fn new(vulkan: &Vulkan, logical: &LogicalDevice) -> Self {
        // Create the buffer allocator. It use the Vulkan Memory Allocator library
        // with rust bindings. The crate is a fork that I made to allow using
        // vulkanalia bindings instead of ash. This may cause some issues...
        let inner = Allocator::new(AllocatorCreateInfo::new(
            vulkan.instance().clone(),
            logical.inner(),
            logical.physical().inner(),
        ))
        .expect("Failed to create buffer allocator");

        Self { inner }
    }

    /// Return the Vulkan Memory Allocator allocator.
    #[must_use]
    pub(crate) fn inner(&self) -> &Allocator {
        &self.inner
    }
}
