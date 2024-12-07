use crate::device::VulkanDevice;
use std::{marker::PhantomData, sync::Arc};
use vulkanalia::prelude::v1_3::*;

/// A command pool. Command pools are used to allocate command buffers. Commands
/// buffers from the same pool must be accessed from the same thread. If you need
/// to use command buffers from multiple threads, you need to create multiple
/// command pools.
#[derive(Debug)]
pub struct CommandPool {
    /// The device that owns the command pool.
    device: Arc<VulkanDevice>,

    /// The vulkan command pool object.
    inner: vk::CommandPool,

    /// A marker to make `CommandPool` non-send, since command buffers from the
    /// same pool must be accessed from the same thread.
    _non_send: PhantomData<*const ()>,
}

impl CommandPool {
    /// Create a new command pool for the given queue family.
    #[must_use]
    pub fn new(device: Arc<VulkanDevice>, queue: u32, flags: vk::CommandPoolCreateFlags) -> Self {
        let info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue)
            .flags(flags.into());

        let inner = unsafe {
            device
                .logical()
                .create_command_pool(&info, None)
                .expect("Failed to create command pool")
        };

        Self {
            device,
            inner,
            _non_send: PhantomData,
        }
    }

    /// Returns the vulkan command pool object.
    #[must_use]
    pub const fn inner(&self) -> vk::CommandPool {
        self.inner
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.logical().destroy_command_pool(self.inner, None);
        }
    }
}

/// A command buffer. Command buffers are used to record commands that will be
/// submitted to the GPU.
#[derive(Debug)]
pub struct CommandBuffer<'pool> {
    /// The device that owns the command buffer.
    device: Arc<VulkanDevice>,

    /// The vulkan command buffer object.
    inner: vk::CommandBuffer,

    /// The command pool that allocated the command buffer.
    pool: &'pool CommandPool,
}

impl<'pool> CommandBuffer<'pool> {
    /// Allocate a new primary command buffer from the given pool.
    #[must_use]
    pub fn new(pool: &'pool CommandPool) -> Self {
        let info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(pool.inner())
            .command_buffer_count(1);

        let inner = unsafe {
            pool.device
                .logical()
                .allocate_command_buffers(&info)
                .expect("Failed to allocate command buffer")[0]
        };

        Self {
            device: Arc::clone(&pool.device),
            inner,
            pool,
        }
    }

    /// Returns the vulkan command buffer object.
    #[must_use]
    pub const fn inner(&self) -> vk::CommandBuffer {
        self.inner
    }
}

impl Drop for CommandBuffer<'_> {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .free_command_buffers(self.pool.inner(), &[self.inner]);
        }
    }
}
