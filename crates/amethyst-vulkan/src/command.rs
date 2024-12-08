use crate::{device::VulkanDevice, pipeline::Pipeline};
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

/// The state of a command buffer.
pub trait State {}

/// The command buffer is idle and does not have a recording pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Idle;
impl State for Idle {}

/// The command buffer is recording commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Recording;
impl State for Recording {}

/// The command buffer has finished recording commands and is ready to be
/// submitted to the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Executable;
impl State for Executable {}

/// A command buffer. Command buffers are used to record commands that will be
/// submitted to the GPU.
#[derive(Debug)]
pub struct CommandBuffer<'pool, T: State = Idle> {
    /// The device that owns the command buffer.
    device: Arc<VulkanDevice>,

    /// The vulkan command buffer object.
    inner: vk::CommandBuffer,

    /// The command pool that allocated the command buffer.
    pool: &'pool CommandPool,

    /// A marker to register the state of the command buffer
    /// during compile time.
    state: PhantomData<T>,
}

impl<T: State> CommandBuffer<'_, T> {
    /// Returns the vulkan command buffer object.
    #[must_use]
    pub const fn inner(&self) -> vk::CommandBuffer {
        self.inner
    }
}

impl<'pool> CommandBuffer<'pool, Idle> {
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
            state: PhantomData,
            inner,
            pool,
        }
    }

    /// Consumes the idle command buffer and starts recording commands. The
    /// command buffer will be in the recording state after this method is called,
    /// allowing you to record commands.
    #[must_use]
    pub fn start_recording(self) -> CommandBuffer<'pool, Recording> {
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .logical()
                .begin_command_buffer(self.inner, &begin_info)
                .expect("Failed to begin command buffer");
        }

        // Encapsulate the command buffer in a ManuallyDrop to change the state
        // without running the destructor.
        let command = std::mem::ManuallyDrop::new(self);
        CommandBuffer {
            device: Arc::clone(&command.device),
            state: PhantomData,
            inner: command.inner,
            pool: command.pool,
        }
    }
}

impl<'pool> CommandBuffer<'pool, Recording> {
    #[must_use]
    pub fn pipeline_barrier(self, info: PipelineBarrierInfo) -> Self {
        let buffers_barriers: [vk::BufferMemoryBarrier; 0] = [];
        let memories_barriers: [vk::MemoryBarrier; 0] = [];
        let images_barriers = info.images_barriers.as_slice();

        unsafe {
            self.device.logical().cmd_pipeline_barrier(
                self.inner,
                info.src_stage_mask.into(),
                info.dst_stage_mask.into(),
                vk::DependencyFlags::empty(),
                &memories_barriers,
                &buffers_barriers,
                &images_barriers,
            );
        }
        self
    }

    /// Bind a graphic pipeline to the command buffer.
    #[must_use]
    pub fn bind_graphic_pipeline(self, pipeline: &Pipeline) -> Self {
        unsafe {
            self.device.logical().cmd_bind_pipeline(
                self.inner,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.inner(),
            );
        }
        self
    }

    /// Start a dynamic render pass instance
    #[must_use]
    pub fn start_rendering(self, info: RenderingInfo) -> Self {
        let render_area = vk::Rect2D::builder()
            .extent(vk::Extent2D::from(info.render_area))
            .build();

        let rendering_info = vk::RenderingInfo::builder()
            .color_attachments(&info.colors_attachements)
            .render_area(render_area)
            .layer_count(1);

        unsafe {
            self.device
                .logical()
                .cmd_begin_rendering(self.inner, &rendering_info);
        }
        self
    }

    /// Draw primitives.
    ///
    /// # Safety
    /// TODO
    #[must_use]
    pub unsafe fn draw(self, info: DrawInfo) -> Self {
        self.device.logical().cmd_draw(
            self.inner,
            info.vertex_count,
            info.instance_count,
            info.first_vertex,
            info.first_instance,
        );
        self
    }

    /// End a dynamic render pass instance
    #[must_use]
    pub fn stop_rendering(self) -> Self {
        unsafe { self.device.logical().cmd_end_rendering(self.inner) }
        self
    }

    /// Stop recording commands and transition the command buffer to the
    /// executable state. The command buffer will be in the executable state after
    /// this method is called, allowing you to submit the command buffer to the
    /// GPU.
    #[must_use]
    pub fn stop_recording(self) -> CommandBuffer<'pool, Executable> {
        unsafe {
            self.device
                .logical()
                .end_command_buffer(self.inner)
                .expect("Failed to end command buffer");
        }

        // Encapsulate the command buffer in a ManuallyDrop to change the state
        // without running the destructor.
        let command = std::mem::ManuallyDrop::new(self);
        CommandBuffer {
            device: Arc::clone(&command.device),
            state: PhantomData,
            inner: command.inner,
            pool: command.pool,
        }
    }
}

impl<'pool> CommandBuffer<'pool, Executable> {
    /// Submit the command buffer to a queue and wait for it to finish executing.
    pub fn submit_and_wait(self, info: SubmitInfo) {
        let commands = [self.inner];
        let submit_info = vk::SubmitInfo::builder()
            .wait_dst_stage_mask(&info.wait_dst_stage_mask)
            .signal_semaphores(&info.signal_semaphores)
            .wait_semaphores(&info.wait_semaphores)
            .command_buffers(&commands);

        unsafe {
            self.device
                .logical()
                .queue_submit(info.queue, &[submit_info], vk::Fence::null())
                .expect("Failed to submit command buffer to graphics queue");
        }

        unsafe {
            self.device
                .logical()
                .queue_wait_idle(info.queue)
                .expect("Failed to wait for graphic queue to finish rendering");
        }
    }
}

impl<T: State> Drop for CommandBuffer<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .free_command_buffers(self.pool.inner(), &[self.inner]);
        }
    }
}

/// A pipeline barrier info.
pub struct PipelineBarrierInfo {
    pub src_stage_mask: vk::PipelineStageFlags,
    pub dst_stage_mask: vk::PipelineStageFlags,
    pub images_barriers: Vec<vk::ImageMemoryBarrier>,
}

/// A rendering info.
pub struct RenderingInfo {
    pub colors_attachements: Vec<vk::RenderingAttachmentInfo>,
    pub render_area: vk::Extent2D,
}

pub struct DrawInfo {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

pub struct SubmitInfo {
    pub queue: vk::Queue,
    pub signal_semaphores: Vec<vk::Semaphore>,
    pub wait_semaphores: Vec<vk::Semaphore>,
    pub wait_dst_stage_mask: Vec<vk::PipelineStageFlags>,
}
