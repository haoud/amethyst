use crate::device::VulkanDevice;
use std::sync::Arc;
use vulkanalia::prelude::v1_3::*;

/// A binary semaphore. It is a GPU-GPU synchronization primitive that can be
/// used to insert a dependency between queue operations or between a queue
/// operation and the host.
/// For example, a semaphore can be used to ensure that an image has finished
/// to be rendered before being presented to the swapchain.
#[derive(Debug)]
pub struct Semaphore {
    device: Arc<VulkanDevice>,
    inner: vk::Semaphore,
}

impl Semaphore {
    /// Creates a new semaphore.
    #[must_use]
    pub fn new(device: Arc<VulkanDevice>) -> Self {
        let info = vk::SemaphoreCreateInfo::builder();
        let inner = unsafe {
            device
                .logical()
                .create_semaphore(&info, None)
                .expect("Failed to create semaphore")
        };

        Self { device, inner }
    }

    /// Return the inner vulkan semaphore.
    #[must_use]
    pub const fn inner(&self) -> vk::Semaphore {
        self.inner
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.logical().destroy_semaphore(self.inner, None);
        }
    }
}

/// A fence is a CPU-GPU synchronization primitive that can be used to insert a
/// dependency from a queue to the host.
#[derive(Debug)]
pub struct Fence {
    device: Arc<VulkanDevice>,
    inner: vk::Fence,
}

impl Fence {
    /// Creates a new fence with the given flags.
    #[must_use]
    pub fn new(device: Arc<VulkanDevice>, flags: vk::FenceCreateFlags) -> Self {
        let info = vk::FenceCreateInfo::builder().flags(flags);
        let inner = unsafe {
            device
                .logical()
                .create_fence(&info, None)
                .expect("Failed to create fence")
        };

        Self { device, inner }
    }

    /// Query the current status of the fence without resetting it nor
    /// waiting for it.
    pub fn query(&self) -> FenceStatus {
        let status = unsafe {
            self.device
                .logical()
                .get_fence_status(self.inner)
                .expect("Failed to query fence status")
        };

        match status {
            vk::SuccessCode::NOT_READY => FenceStatus::Unsignaled,
            vk::SuccessCode::SUCCESS => FenceStatus::Signaled,
            _ => panic!("Unexpected fence status: {:?}", status),
        }
    }

    /// Reset the fence to the unsignaled state.
    pub fn reset(&self) {
        unsafe {
            self.device
                .logical()
                .reset_fences(&[self.inner])
                .expect("Failed to reset fence");
        }
    }

    /// Wait for the fence to be signaled. This function will block the current
    /// thread until the fence is signaled without a timeout.
    pub fn wait(&self) {
        unsafe {
            self.device
                .logical()
                .wait_for_fences(&[self.inner], true, u64::MAX)
                .expect("Failed to wait for fence");
        }
    }

    /// Return the inner vulkan fence.
    pub const fn inner(&self) -> vk::Fence {
        self.inner
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.device.logical().destroy_fence(self.inner, None);
        }
    }
}

/// The status of a fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FenceStatus {
    Unsignaled,
    Signaled,
}
