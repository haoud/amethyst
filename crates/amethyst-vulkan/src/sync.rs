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

/// A fence is a CPU-GPU synchronization primitive that can be used to insert a
/// dependency from a queue to the host.
pub struct Fence {
    device: Arc<RenderDevice>,
    inner: vk::Fence,
}

impl Fence {
    /// Creates a new fence with the given flags.
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, flags: FenceCreateFlags) -> Self {
        let info = vk::FenceCreateInfo::builder()
            .flags(flags.into())
            .build();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_fence(&info, None)
                .expect("Failed to create fence")
        };

        Self { device, inner }
    }

    /// Query the current status of the fence without resetting it no
    /// waiting for it.
    pub fn query(&self) -> FenceStatus {
        let status = unsafe {
            self.device
                .logical()
                .inner()
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
                .inner()
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
                .inner()
                .wait_for_fences(&[self.inner], true, u64::MAX)
                .expect("Failed to wait for fence");
        }
    }

    /// Return the inner vulkan fence.
    pub(crate) fn inner(&self) -> vk::Fence {
        self.inner
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_fence(self.inner, None);
        }
    }
}

/// Configuration used to create a fence.
pub struct FenceCreateFlags {
    /// Specifies that the fence object is created in the signaled state.
    /// Otherwise, it is created in the unsignaled state.
    /// If a fence is created in the signaled state, applications can wait for
    /// it without delay assuming they have not reset it since it was signaled.
    pub signaled: bool,
}

impl Default for FenceCreateFlags {
    fn default() -> Self {
        Self { signaled: false }
    }
}

impl From<FenceCreateFlags> for vk::FenceCreateFlags {
    fn from(infos: FenceCreateFlags) -> Self {
        let mut flags = vk::FenceCreateFlags::empty();
        if infos.signaled {
            flags |= vk::FenceCreateFlags::SIGNALED;
        }
        flags
    }
}

/// The status of a fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FenceStatus {
    Unsignaled,
    Signaled,
}
