use crate::{
    buffer::subbuffer::SubBuffer,
    descriptor::DescriptorSet,
    device::RenderDevice,
    prelude::{
        AttachmentLoadOp, AttachmentStoreOp, Image, ImageAccess, ImageLayout,
        ImageSubResourceLayer, ImageSubResourceRange, ImageView, Pipeline, PipelineStage,
    },
    surface::{Extent2D, Extent3D},
};
use std::{marker::PhantomData, mem::ManuallyDrop, sync::Arc};
use vulkanalia::{prelude::v1_2::*, vk::DeviceV1_3};

pub mod pool;

/// A command buffer. Command buffers are used to record commands that will be
/// executed by the GPU.
pub struct Command<T: State = Idle> {
    device: Arc<RenderDevice>,
    inner: vk::CommandBuffer,
    _state: PhantomData<T>,
}

impl<T: State> Command<T> {
    pub(crate) fn inner(&self) -> vk::CommandBuffer {
        self.inner
    }
}

impl Command<Idle> {
    /// Create a new command buffer.
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, _: CommandCreateInfo) -> Self {
        let info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(device.commands_pool().inner())
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let inner = unsafe {
            device
                .logical()
                .inner()
                .allocate_command_buffers(&info)
                .expect("Failed to allocate command buffer")[0]
        };

        Self {
            device,
            inner,
            _state: PhantomData,
        }
    }

    /// Start recording commands. If you want to record commands, you need to
    /// call this method first.
    #[must_use]
    pub fn start_recording(self) -> Command<Recording> {
        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .logical()
                .inner()
                .begin_command_buffer(self.inner, &info)
                .expect("Failed to begin command buffer");
        }

        let command = ManuallyDrop::new(self);
        Command::<Recording> {
            device: command.device.clone(),
            inner: command.inner,
            _state: PhantomData,
        }
    }
}

impl Command<Recording> {
    /// Put a pipeline barrier. Pipeline barriers are used to synchronize resources between
    /// commands. For example, pipeline barrier are used during dynamic rendering to
    /// synchronize the image layout of the swapchain images: the swapchain images are
    /// initially in the `PRESENT_SRC_KHR` layout, but we need to transition them to the
    /// `COLOR_ATTACHMENT_OPTIMAL` layout before we can render to them, and then transition
    /// them back to the `PRESENT_SRC_KHR` layout before we can present them.
    pub fn pipeline_barrier(self, info: PipelineBarrierInfo) -> Self {
        let images_barriers = info
            .images_barriers
            .into_iter()
            .map(|barrier| vk::ImageMemoryBarrier::from(barrier))
            .collect::<Vec<_>>();

        let buffers_barriers: [vk::BufferMemoryBarrier; 0] = [];
        let memories_barriers: [vk::MemoryBarrier; 0] = [];

        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_pipeline_barrier(
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

    /// Bind a vertex buffer. This must be done before drawing.
    pub fn bind_vertex_buffers<T>(self, buffer: &SubBuffer<T>) -> Self {
        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_bind_vertex_buffers(self.inner, 0, &[buffer.inner()], &[0]);
        }
        self
    }

    /// Bind an index buffer.
    pub fn bind_indices_buffers<T>(self, buffer: &SubBuffer<T>, kind: IndicesType) -> Self {
        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_bind_index_buffer(self.inner, buffer.inner(), 0, vk::IndexType::from(kind));
        }
        self
    }

    /// Bind a pipeline. This must be done before drawing.
    pub fn bind_graphics_pipeline(self, pipeline: &Pipeline) -> Self {
        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_bind_pipeline(
                    self.inner,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.inner(),
                );
        }
        self
    }

    /// Bind descriptor sets. This must be done before drawing.
    pub fn bind_descriptor_sets(
        self,
        pipeline: &Pipeline,
        descriptor_sets: &[&DescriptorSet],
    ) -> Self {
        let sets = descriptor_sets
            .iter()
            .map(|set| set.inner())
            .collect::<Vec<_>>();

        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_bind_descriptor_sets(
                    self.inner,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.layout(),
                    0,
                    &sets,
                    &[],
                )
        }
        self
    }

    /// Start rendering. This must be done before drawing.
    pub fn start_rendering(self, info: RenderingInfo) -> Self {
        let colors_attachements = info
            .colors_attachements
            .into_iter()
            .map(|info| vk::RenderingAttachmentInfo::from(info))
            .collect::<Vec<_>>();

        let depth_attachement = info
            .depth_attachement
            .map(|info| vk::RenderingAttachmentInfo::from(info));

        let render_area = vk::Rect2D::builder()
            .extent(vk::Extent2D::from(info.render_area))
            .build();

        let builder = vk::RenderingInfo::builder()
            .color_attachments(&colors_attachements)
            .render_area(render_area)
            .layer_count(1);

        let rendering_info = if let Some(depth_attachement) = depth_attachement.as_ref() {
            builder
                .depth_attachment(depth_attachement)
                .build()
        } else {
            builder.build()
        };

        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_begin_rendering(self.inner, &rendering_info);
        }
        self
    }

    /// Draw
    pub fn draw(self, info: DrawCommandInfo) -> Self {
        unsafe {
            self.device.logical().inner().cmd_draw(
                self.inner,
                info.vertex_count,
                info.instance_count,
                info.first_vertex,
                info.first_instance,
            );
        }
        self
    }

    pub fn draw_indexed(self, info: DrawIndexedCommandInfo) -> Self {
        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_draw_indexed(
                    self.inner,
                    info.index_count,
                    info.instance_count,
                    info.first_index,
                    0,
                    info.first_instance,
                );
        }
        self
    }

    /// End rendering. This must be done after drawing.
    pub fn end_rendering(self) -> Self {
        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_end_rendering(self.inner)
        }
        self
    }

    /// Copy a buffer to another buffer.
    pub fn copy_buffer<T>(self, info: CopyBufferInfo<T>) -> Self {
        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_copy_buffer(
                    self.inner,
                    info.src.inner(),
                    info.dst.inner(),
                    &[vk::BufferCopy::builder()
                        .dst_offset(info.dst.offset() as u64)
                        .src_offset(info.src.offset() as u64)
                        .size(info.size)
                        .build()],
                );
        }
        self
    }

    /// Update a buffer with the given data. It first copy the data into command buffer memory
    /// when the command is recorded (which requires additional storage and may incur an additional
    /// allocation), and then copy the data from the command buffer into dstBuffer when the command
    /// is executed on a device.
    ///
    /// The additional cost of this functionality compared to buffer to buffer copies means it is only
    /// recommended for very small amounts of data, and is why it is limited to only 65536 bytes.
    ///
    /// # Panics
    /// Panics if the data size is smaller than the buffer size or if the data size is greater than
    /// 65536 bytes.
    pub fn update_buffer<T>(self, buffer: &SubBuffer<T>, data: &[T]) -> Self {
        assert!((data.len() * std::mem::size_of::<T>()) >= buffer.size());
        assert!(data.len() <= 65536);

        unsafe {
            let data = std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * std::mem::size_of::<T>(),
            );

            self.device
                .logical()
                .inner()
                .cmd_update_buffer(self.inner, buffer.inner(), 0, data);
        }
        self
    }

    pub fn copy_buffer_to_image(
        self,
        buffer: &SubBuffer<u8>,
        image: &Image,
        info: CopyBufferIntoImageInfo,
    ) -> Self {
        let subressource = vk::ImageSubresourceLayers::from(info.subresource_layer);
        let extent = vk::Extent3D::from(info.extent);
        let offset = vk::Offset3D { x: 0, y: 0, z: 0 };

        let region = vk::BufferImageCopy::builder()
            .image_subresource(subressource)
            .image_extent(extent)
            .image_offset(offset)
            .buffer_image_height(0)
            .buffer_row_length(0)
            .buffer_offset(0)
            .build();

        unsafe {
            self.device
                .logical()
                .inner()
                .cmd_copy_buffer_to_image(
                    self.inner,
                    buffer.inner(),
                    image.inner(),
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[region],
                );
        }

        self
    }

    /// Stop recording commands. If you want to submit the commands, you need to
    /// call this method first.
    pub fn stop_recording(self) -> Command<Executable> {
        unsafe {
            self.device
                .logical()
                .inner()
                .end_command_buffer(self.inner)
                .expect("Failed to end command buffer");
        }

        let command = ManuallyDrop::new(self);
        Command::<Executable> {
            device: command.device.clone(),
            inner: command.inner,
            _state: PhantomData,
        }
    }
}

impl Command<Executable> {}

impl<T: State> Drop for Command<T> {
    fn drop(&mut self) {
        unsafe {
            let pool = self.device.commands_pool().inner();

            self.device
                .logical()
                .inner()
                .free_command_buffers(pool, &[self.inner]);
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
/// submitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Executable;
impl State for Executable {}

/// A command create info.
pub struct CommandCreateInfo {}

impl Default for CommandCreateInfo {
    fn default() -> Self {
        Self {}
    }
}

/// The data type of the indices.
pub enum IndicesType {
    U16,
    U32,
}

impl From<IndicesType> for vk::IndexType {
    fn from(x: IndicesType) -> Self {
        match x {
            IndicesType::U16 => vk::IndexType::UINT16,
            IndicesType::U32 => vk::IndexType::UINT32,
        }
    }
}

/// A pipeline barrier info.
pub struct PipelineBarrierInfo<'a> {
    pub src_stage_mask: PipelineStage,
    pub dst_stage_mask: PipelineStage,
    pub images_barriers: Vec<ImageBarrier<'a>>,
}

/// An image barrier. Image barriers are used to synchronize images between
/// commands.
///
/// For example, image barriers are used during dynamic rendering to synchronize the image
/// layout of the swapchain images: the swapchain images are initially in the `PRESENT_SRC_KHR`
/// layout, but we need to transition them to the `COLOR_ATTACHMENT_OPTIMAL` layout before we
/// can render to them, and then transition them back to the `PRESENT_SRC_KHR` layout before we
/// can present them.
pub struct ImageBarrier<'a> {
    pub subresource_range: ImageSubResourceRange,
    pub src_access_mask: ImageAccess,
    pub dst_access_mask: ImageAccess,
    pub old_layout: ImageLayout,
    pub new_layout: ImageLayout,
    pub image: &'a Image,
}

impl From<ImageBarrier<'_>> for vk::ImageMemoryBarrier {
    fn from(barrier: ImageBarrier) -> Self {
        vk::ImageMemoryBarrier::builder()
            .subresource_range(vk::ImageSubresourceRange::from(barrier.subresource_range))
            .src_access_mask(barrier.src_access_mask.into())
            .dst_access_mask(barrier.dst_access_mask.into())
            .old_layout(barrier.old_layout.into())
            .new_layout(barrier.new_layout.into())
            .image(barrier.image.inner())
            .build()
    }
}

/// A rendering info.
pub struct RenderingInfo<'a> {
    pub depth_attachement: Option<RenderingAttachementInfo<'a>>,
    pub colors_attachements: Vec<RenderingAttachementInfo<'a>>,
    pub render_area: Extent2D,
}

/// A rendering attachement info.
pub struct RenderingAttachementInfo<'a> {
    pub image_view: &'a ImageView,
    pub image_layout: ImageLayout,
    pub load_op: AttachmentLoadOp,
    pub store_op: AttachmentStoreOp,
    pub clear_value: ClearValue,
}

impl From<RenderingAttachementInfo<'_>> for vk::RenderingAttachmentInfo {
    fn from(info: RenderingAttachementInfo) -> Self {
        vk::RenderingAttachmentInfo::builder()
            .image_layout(info.image_layout.into())
            .image_view(info.image_view.inner())
            .clear_value(info.clear_value.into())
            .store_op(info.store_op.into())
            .load_op(info.load_op.into())
            .build()
    }
}
pub enum ClearValue {
    Color([f32; 4]),
    DepthStencil(f32, u32),
}

impl From<ClearValue> for vk::ClearValue {
    fn from(x: ClearValue) -> Self {
        match x {
            ClearValue::Color(colors) => vk::ClearValue {
                color: vk::ClearColorValue { float32: colors },
            },
            ClearValue::DepthStencil(depth, stencil) => vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue { depth, stencil },
            },
        }
    }
}

/// A draw command info.
pub struct DrawCommandInfo {
    /// The number of instances to draw.
    pub instance_count: u32,

    /// The index of the first instance to draw.
    pub first_instance: u32,

    /// The number of vertices to draw.
    pub vertex_count: u32,

    /// The index of the first vertex to draw.
    pub first_vertex: u32,
}

/// A indexed draw command info.
pub struct DrawIndexedCommandInfo {
    /// The number of instances to draw.
    pub instance_count: u32,

    /// The index of the first instance to draw.
    pub first_instance: u32,

    /// The number of vertices to draw.
    pub index_count: u32,

    /// The index of the first vertex to draw.
    pub first_index: u32,
}

pub struct CopyBufferInfo<'a, T> {
    /// The source buffer.
    pub src: &'a SubBuffer<T>,

    /// The destination buffer.
    pub dst: &'a SubBuffer<T>,

    /// The size of the data to copy.
    pub size: u64,
}

pub struct CopyBufferIntoImageInfo {
    pub subresource_layer: ImageSubResourceLayer,
    pub extent: Extent3D,
}
