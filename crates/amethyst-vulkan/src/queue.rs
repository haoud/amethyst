use crate::{
    command::{Command, Executable},
    device::{LogicalDevice, RenderDevice},
    sync::Semaphore,
};
use vulkanalia::prelude::v1_2::*;

/// A queue index within a device queue list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueueIndex(pub u32);

impl QueueIndex {
    #[must_use]
    pub fn new(index: u32) -> Self {
        Self(index)
    }
}

/// A queue within a logical device. It contains a Vulkan queue and its
/// index within the device to avoid fetching it from the device each
/// time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Queue {
    index: QueueIndex,
    inner: vk::Queue,
}

impl Queue {
    /// Creates a new queue.
    #[must_use]
    pub(crate) fn new(inner: vk::Queue, index: QueueIndex) -> Self {
        Self { index, inner }
    }

    /// Fetches the first queue from the specified family index.
    #[must_use]
    pub fn fetch(logical: &LogicalDevice, family_index: u32) -> Self {
        let index = QueueIndex::new(family_index);
        let inner = unsafe {
            logical
                .inner()
                .get_device_queue(family_index, 0)
        };

        Self { index, inner }
    }

    /// Submits commands into the queue.
    pub fn submit(&self, device: &RenderDevice, info: QueueSubmitInfo) {
        assert!(info.signal_semaphore.len() == info.wait_semaphore.len());

        // TODO: Make this configurable ? How ?
        let wait_stages = (0..info.wait_semaphore.len())
            .map(|_| vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .collect::<Vec<_>>();

        let signal_semaphores = info
            .signal_semaphore
            .iter()
            .map(|semaphore| semaphore.inner())
            .collect::<Vec<_>>();

        let wait_semaphores = info
            .wait_semaphore
            .iter()
            .map(|semaphore| semaphore.inner())
            .collect::<Vec<_>>();

        let commands = info
            .commands
            .iter()
            .map(|command| command.inner())
            .collect::<Vec<_>>();

        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&wait_stages)
            .signal_semaphores(&signal_semaphores)
            .wait_semaphores(&wait_semaphores)
            .command_buffers(&commands)
            .build();

        unsafe {
            device
                .logical()
                .inner()
                .queue_submit(self.inner, &[submit_info], vk::Fence::null())
                .expect("Failed to submit queue");
        }
    }

    /// Waits for the queue to finish executing commands.
    pub fn wait_idle(&self, device: &RenderDevice) {
        unsafe {
            device
                .logical()
                .inner()
                .queue_wait_idle(self.inner)
                .expect("Failed to wait for queue");
        }
    }

    /// Returns the queue index within device used to create this queue.
    #[must_use]
    pub fn family_index(&self) -> QueueIndex {
        self.index
    }

    /// Returns the Vulkan queue.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::Queue {
        self.inner
    }
}

/// A structure that contains the information needed to submit commands
/// into a queue.
pub struct QueueSubmitInfo<'a> {
    /// The semaphore that will be signaled when the queue finishes
    /// executing the commands.
    pub signal_semaphore: &'a [&'a Semaphore],

    /// The semaphore that the queue will wait on before executing the
    /// commands.
    pub wait_semaphore: &'a [&'a Semaphore],

    /// The commands to submit.
    pub commands: &'a [&'a Command<Executable>],
}
