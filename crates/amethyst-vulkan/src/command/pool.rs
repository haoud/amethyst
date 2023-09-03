use crate::{device::LogicalDevice, prelude::Queue};
use bitflags::bitflags;
use std::sync::Arc;
use vulkanalia::prelude::v1_2::*;

/// A command pool. Command pools are used to allocate command buffers. Commands
/// buffers from the same pool must be accessed from the same thread. If you need
/// to use command buffers from multiple threads, you need to create multiple
/// command pools.
pub struct CommandPool {
    device: Arc<LogicalDevice>,
    inner: vk::CommandPool,
}

impl CommandPool {
    /// Create a new command pool for the given queue.
    #[must_use]
    pub fn new(device: Arc<LogicalDevice>, queue: &Queue, flags: CommandPoolCreateFlags) -> Self {
        let info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue.family_index().0)
            .flags(flags.into());

        let inner = unsafe {
            device
                .inner()
                .create_command_pool(&info, None)
                .expect("Failed to create command pool")
        };

        Self { device, inner }
    }

    /// Returns the vulkan command pool object.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::CommandPool {
        self.inner
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_command_pool(self.inner, None);
        }
    }
}

bitflags! {
    pub struct CommandPoolCreateFlags : u64 {
        const TRANSIANT = 1 << 0;
        const PROTECTED = 1 << 1;
        const MAY_RESET = 1 << 2;
    }
}

impl From<vk::CommandPoolCreateFlags> for CommandPoolCreateFlags {
    fn from(x: vk::CommandPoolCreateFlags) -> Self {
        let mut flags = Self::empty();
        if x.contains(vk::CommandPoolCreateFlags::TRANSIENT) {
            flags = flags | Self::TRANSIANT;
        }
        if x.contains(vk::CommandPoolCreateFlags::PROTECTED) {
            flags = flags | Self::PROTECTED;
        }
        if x.contains(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER) {
            flags = flags | Self::MAY_RESET;
        }
        flags
    }
}

impl From<CommandPoolCreateFlags> for vk::CommandPoolCreateFlags {
    fn from(x: CommandPoolCreateFlags) -> Self {
        let mut flags = Self::empty();
        if x.contains(CommandPoolCreateFlags::TRANSIANT) {
            flags = flags | Self::TRANSIENT;
        }
        if x.contains(CommandPoolCreateFlags::PROTECTED) {
            flags = flags | Self::PROTECTED;
        }
        if x.contains(CommandPoolCreateFlags::MAY_RESET) {
            flags = flags | Self::RESET_COMMAND_BUFFER;
        }
        flags
    }
}
