use crate::device::RenderDevice;
use std::sync::Arc;
use vulkanalia::prelude::v1_2::*;

/// A binary semaphore. It is a GPU-GPU synchronization primitive that can be
/// used to insert a dependency between queue operations or between a queue
/// operation and the host.
/// For example, a semaphore can be used to ensure that an image has finished
/// to be rendered before being presented to the swapchain.
pub struct Semaphore {
    device: Arc<RenderDevice>,
    inner: vk::Semaphore,
}

impl Semaphore {
    /// Creates a new semaphore.
    #[must_use]
    pub fn new(device: Arc<RenderDevice>) -> Self {
        let info = vk::SemaphoreCreateInfo::builder();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_semaphore(&info, None)
                .expect("Failed to create semaphore")
        };

        Self { device, inner }
    }

    /// Return the inner vulkan semaphore.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::Semaphore {
        self.inner
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_semaphore(self.inner, None);
        }
    }
}
