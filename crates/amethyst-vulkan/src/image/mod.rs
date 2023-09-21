use crate::{
    buffer::{
        subbuffer::{SubBuffer, SubBufferCreateInfo},
        Buffer, BufferAccessMode, BufferCreateInfo, BufferKind, BufferMemoryLocation,
        BufferTransfert, BufferUsageInfo,
    },
    command::{
        Command, CommandCreateInfo, CommandSubmitInfo, CopyBufferIntoImageInfo, ImageBarrier,
        ImageBlit, ImageBlitInfo, PipelineBarrierInfo,
    },
    device::RenderDevice,
    prelude::PipelineStage,
    Offset3D,
};
use bitflags::bitflags;
use std::{ops::Add, sync::Arc};
pub use vulkanalia::prelude::v1_2::*;

use self::{sampler::ImageSampler, view::ImageView};

/// An image format. This is a redefinition of the `Format` enum
pub type ImageFormat = crate::format::Format;

pub mod sampler;
pub mod surface;
pub mod view;

/// An image
pub struct Image {
    mip_levels: u32,
    memory: ImageMemory,
    inner: vk::Image,
}

impl Image {
    /// Create a new empty image with the given create info. The image will be created with
    /// the `UNDEFINED` layout and can be filled with data using a command buffer.
    #[must_use]
    pub fn empty(device: Arc<RenderDevice>, info: ImageCreateInfo) -> Self {
        assert!(info.extent.width > 0 && info.extent.height > 0);

        let extent = vk::Extent3D::builder()
            .height(info.extent.height)
            .width(info.extent.width)
            .depth(1)
            .build();

        // Compute the number of mipmaps
        let mip_level = match info.mipmap_levels {
            MipmapLevel::Count(count) => count,
            MipmapLevel::None => 1,
            MipmapLevel::Auto => {
                let height = info.extent.height as f32;
                let width = info.extent.width as f32;
                width.max(height).log2().floor().add(1.0) as u32
            }
        };

        let create_info = vk::ImageCreateInfo::builder()
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .image_type(vk::ImageType::_2D)
            .format(info.format.into())
            .mip_levels(mip_level)
            .usage(info.usage.into())
            .array_layers(1)
            .extent(extent)
            .build();

        // Create the image
        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_image(&create_info, None)
                .expect("Failed to create image")
        };

        // Get the required memory type for the image
        let memory_requirements = unsafe {
            device
                .logical()
                .inner()
                .get_image_memory_requirements(inner)
        };

        let buffer = Buffer::empty(
            Arc::clone(&device),
            BufferCreateInfo {
                usage: BufferUsageInfo {
                    memory_type: memory_requirements.memory_type_bits,
                    location: BufferMemoryLocation::PreferHostVisible,
                    transfer: BufferTransfert::Destination,
                    access: BufferAccessMode::Sequential,
                },
                alignment: memory_requirements.alignment as usize,
                size: memory_requirements.size as usize,
                kind: BufferKind::None,
            },
        );

        // Bind the image memory
        unsafe {
            device
                .logical()
                .inner()
                .bind_image_memory(inner, buffer.device_memory(), buffer.device_memory_offset())
                .expect("Failed to bind image memory");
        }

        Self {
            memory: ImageMemory::Buffer(SubBuffer::from(buffer)),
            mip_levels: mip_level,
            inner,
        }
    }

    /// Create a new image with the given create info and will be filled with
    /// the given data. This function will block until the image is fully created.
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, data: &[u8], info: ImageCreateInfo) -> Self {
        let image = Image::empty(Arc::clone(&device), info.clone());

        if data.is_empty() {
            return image;
        }

        let staging = SubBuffer::new(
            Arc::clone(&device),
            data,
            SubBufferCreateInfo {
                usage: BufferUsageInfo::STAGING,
                kind: BufferKind::None,
                count: data.len(),
                ..Default::default()
            },
        );

        let extent_3d = Extent3D {
            height: info.extent.height,
            width: info.extent.width,
            depth: 1,
        };

        let mut command = Command::new(
            Arc::clone(&device),
            CommandCreateInfo {
                ..Default::default()
            },
        )
        .start_recording()
        .pipeline_barrier(PipelineBarrierInfo {
            src_stage_mask: PipelineStage::TOP_OF_PIPE,
            dst_stage_mask: PipelineStage::TRANSFER,
            images_barriers: vec![ImageBarrier {
                subresource_range: ImageSubResourceRange {
                    level_count: image.mip_levels,
                    ..Default::default()
                },
                src_access_mask: ImageAccess::UNDEFINED,
                dst_access_mask: ImageAccess::TRANSFER_WRITE,
                old_layout: ImageLayout::Undefined,
                new_layout: ImageLayout::TransfertDstOptimal,
                image: &image,
            }],
        })
        .copy_buffer_to_image(
            &staging,
            &image,
            CopyBufferIntoImageInfo {
                subresource_layer: ImageSubResourceLayer::default(),
                extent: extent_3d,
            },
        );

        // Generate the mipmaps if the mip map level is set to auto
        match info.mipmap_levels {
            MipmapLevel::Auto => {
                let mut current_height = info.extent.height as f32;
                let mut current_width = info.extent.width as f32;

                // TODO: Check if the image format supports linear blitting

                for i in 1..image.mip_levels {
                    // Comptute the start and end offset of the blit
                    let src_offset = [
                        Offset3D { x: 0, y: 0, z: 0 },
                        Offset3D {
                            x: current_height as i32,
                            y: current_width as i32,
                            z: 1,
                        },
                    ];

                    // Divide the current height and width by 2 in order to compute the
                    // size of the next mipmap level. If the current height or width is
                    // less than 1, then we don't need to generate the mipmap level.
                    if current_height > 1.0 {
                        current_height /= 2.0;
                    }
                    if current_width > 1.0 {
                        current_width /= 2.0;
                    }

                    // Comptute the start and end offset of the blit
                    let dst_offset = [
                        Offset3D { x: 0, y: 0, z: 0 },
                        Offset3D {
                            x: current_height as i32,
                            y: current_width as i32,
                            z: 1,
                        },
                    ];

                    command = command
                        .pipeline_barrier(PipelineBarrierInfo {
                            src_stage_mask: PipelineStage::TRANSFER,
                            dst_stage_mask: PipelineStage::TRANSFER,
                            images_barriers: vec![ImageBarrier {
                                subresource_range: ImageSubResourceRange {
                                    base_mip_level: i - 1,
                                    ..Default::default()
                                },
                                src_access_mask: ImageAccess::TRANSFER_WRITE,
                                dst_access_mask: ImageAccess::TRANSFER_READ,
                                old_layout: ImageLayout::TransfertDstOptimal,
                                new_layout: ImageLayout::TransfertSrcOptimal,
                                image: &image,
                            }],
                        })
                        .blit_image(ImageBlitInfo {
                            blits: vec![ImageBlit {
                                src_subresource: ImageSubResourceLayer {
                                    aspect_mask: ImageAspectFlags::COLOR,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                    mip_level: i - 1,
                                },
                                src_offsets: src_offset,
                                dst_subresource: ImageSubResourceLayer {
                                    aspect_mask: ImageAspectFlags::COLOR,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                    mip_level: i,
                                },
                                dst_offsets: dst_offset,
                            }],
                            src_image: &image,
                            dst_image: &image,
                            src_image_layout: ImageLayout::TransfertSrcOptimal,
                            dst_image_layout: ImageLayout::TransfertDstOptimal,
                            filter: MipmapFilter::Linear,
                        })
                        .pipeline_barrier(PipelineBarrierInfo {
                            src_stage_mask: PipelineStage::TRANSFER,
                            dst_stage_mask: PipelineStage::FRAGMENT_SHADER,
                            images_barriers: vec![ImageBarrier {
                                subresource_range: ImageSubResourceRange {
                                    base_mip_level: i - 1,
                                    ..Default::default()
                                },
                                src_access_mask: ImageAccess::TRANSFER_READ,
                                dst_access_mask: ImageAccess::SHADER_READ,
                                old_layout: ImageLayout::TransfertSrcOptimal,
                                new_layout: ImageLayout::ShaderReadOnlyOptimal,
                                image: &image,
                            }],
                        });
                }

                // Change the layout of the last mipmap level to the fragment shader read only layout
                command = command.pipeline_barrier(PipelineBarrierInfo {
                    src_stage_mask: PipelineStage::TRANSFER,
                    dst_stage_mask: PipelineStage::FRAGMENT_SHADER,
                    images_barriers: vec![ImageBarrier {
                        subresource_range: ImageSubResourceRange {
                            base_mip_level: image.mip_levels - 1,
                            ..Default::default()
                        },
                        src_access_mask: ImageAccess::TRANSFER_READ,
                        dst_access_mask: ImageAccess::SHADER_READ,
                        old_layout: ImageLayout::TransfertDstOptimal,
                        new_layout: ImageLayout::ShaderReadOnlyOptimal,
                        image: &image,
                    }],
                });
            }
            MipmapLevel::None => {
                // Change the layout of the image to the fragment shader read only layout
                command = command.pipeline_barrier(PipelineBarrierInfo {
                    src_stage_mask: PipelineStage::TRANSFER,
                    dst_stage_mask: PipelineStage::FRAGMENT_SHADER,
                    images_barriers: vec![ImageBarrier {
                        subresource_range: ImageSubResourceRange::default(),
                        src_access_mask: ImageAccess::TRANSFER_WRITE,
                        dst_access_mask: ImageAccess::SHADER_READ,
                        old_layout: ImageLayout::TransfertDstOptimal,
                        new_layout: ImageLayout::ShaderReadOnlyOptimal,
                        image: &image,
                    }],
                });
            }
            MipmapLevel::Count(_) => panic!("Custom mipmaps are not supported yet"),
        }

        command.stop_recording().submit_to(
            device.graphic_queue(),
            CommandSubmitInfo {
                signal_semaphore: &[],
                wait_semaphore: &[],
            },
        );
        image
    }

    /// Create a new image.
    #[must_use]
    pub(crate) fn raw(inner: vk::Image, memory: ImageMemory, mip_levels: u32) -> Self {
        Self {
            inner,
            memory,
            mip_levels,
        }
    }

    /// Return the number of mipmaps levels in the image.
    pub fn mipmap_levels(&self) -> u32 {
        self.mip_levels
    }

    /// Return the inner vulkan image.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::Image {
        self.inner
    }

    /// Return the memory backing the image.
    #[must_use]
    pub fn memory(&self) -> &ImageMemory {
        &self.memory
    }
}

pub struct ImageDescriptorInfo<'a> {
    pub sampler: &'a ImageSampler,
    pub layout: ImageLayout,
    pub view: &'a ImageView,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ImageAccess: u32 {
        const UNDEFINED = vk::AccessFlags::empty().bits();
        const TRANSFER_READ = vk::AccessFlags::TRANSFER_READ.bits();
        const TRANSFER_WRITE = vk::AccessFlags::TRANSFER_WRITE.bits();
        const SHADER_READ = vk::AccessFlags::SHADER_READ.bits();
        const SHADER_WRITE = vk::AccessFlags::SHADER_WRITE.bits();
        const COLOR_ATTACHMENT_WRITE = vk::AccessFlags::COLOR_ATTACHMENT_WRITE.bits();
        const DEPTH_STENCIL_ATTACHMENT_WRITE = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE.bits();
        const DEPTH_STENCIL_ATTACHMENT_READ = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ.bits();

    }
}

impl From<ImageAccess> for vk::AccessFlags {
    fn from(value: ImageAccess) -> Self {
        vk::AccessFlags::from_bits_truncate(value.bits())
    }
}

/// An image layout.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageLayout {
    #[default]
    Undefined,
    AttachmentOptimal,
    ColorAttachmentOptimal,
    PresentSrcKhr,
    ShaderReadOnlyOptimal,
    TransfertSrcOptimal,
    TransfertDstOptimal,
    DepthStencilAttachmentOptimal,
}

impl From<ImageLayout> for vk::ImageLayout {
    fn from(value: ImageLayout) -> Self {
        match value {
            ImageLayout::Undefined => vk::ImageLayout::UNDEFINED,
            ImageLayout::AttachmentOptimal => vk::ImageLayout::ATTACHMENT_OPTIMAL,
            ImageLayout::ColorAttachmentOptimal => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ImageLayout::PresentSrcKhr => vk::ImageLayout::PRESENT_SRC_KHR,
            ImageLayout::ShaderReadOnlyOptimal => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ImageLayout::TransfertSrcOptimal => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            ImageLayout::TransfertDstOptimal => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::DepthStencilAttachmentOptimal => {
                vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL
            }
        }
    }
}

pub enum ImageMemory {
    /// The image is backed by host or device memory.
    Buffer(SubBuffer<u8>),

    /// The image is backed by a swapchain.
    Swapchain,

    Undefined,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ImageAspectFlags : u32 {
        const COLOR = vk::ImageAspectFlags::COLOR.bits();
        const DEPTH = vk::ImageAspectFlags::DEPTH.bits();
    }
}

/// An image subresource range.
pub struct ImageSubResourceRange {
    pub aspect_mask: ImageAspectFlags,
    pub base_array_layer: u32,
    pub base_mip_level: u32,
    pub level_count: u32,
    pub layer_count: u32,
}

impl Default for ImageSubResourceRange {
    fn default() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_array_layer: 0,
            base_mip_level: 0,
            level_count: 1,
            layer_count: 1,
        }
    }
}

impl From<ImageSubResourceRange> for vk::ImageSubresourceRange {
    fn from(value: ImageSubResourceRange) -> Self {
        let aspect_mask = vk::ImageAspectFlags::from_bits_truncate(value.aspect_mask.bits());

        Self {
            base_array_layer: value.base_array_layer,
            base_mip_level: value.base_mip_level,
            level_count: value.level_count,
            layer_count: value.layer_count,
            aspect_mask: aspect_mask,
        }
    }
}

/// An image subresource layer.
pub struct ImageSubResourceLayer {
    pub aspect_mask: ImageAspectFlags,
    pub base_array_layer: u32,
    pub layer_count: u32,
    pub mip_level: u32,
}

impl Default for ImageSubResourceLayer {
    fn default() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_array_layer: 0,
            layer_count: 1,
            mip_level: 0,
        }
    }
}

impl From<ImageSubResourceLayer> for vk::ImageSubresourceLayers {
    fn from(value: ImageSubResourceLayer) -> Self {
        let aspect_mask = vk::ImageAspectFlags::from_bits_truncate(value.aspect_mask.bits());
        Self {
            base_array_layer: value.base_array_layer,
            layer_count: value.layer_count,
            mip_level: value.mip_level,
            aspect_mask: aspect_mask,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ImageUsage: u32 {
        const TRANSFER_SRC = vk::ImageUsageFlags::TRANSFER_SRC.bits();
        const TRANSFER_DST = vk::ImageUsageFlags::TRANSFER_DST.bits();
        const SAMPLED = vk::ImageUsageFlags::SAMPLED.bits();
        const STORAGE = vk::ImageUsageFlags::STORAGE.bits();
        const COLOR_ATTACHMENT = vk::ImageUsageFlags::COLOR_ATTACHMENT.bits();
        const DEPTH_STENCIL_ATTACHMENT = vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT.bits();
    }
}

impl From<ImageUsage> for vk::ImageUsageFlags {
    fn from(value: ImageUsage) -> Self {
        vk::ImageUsageFlags::from_bits_truncate(value.bits())
    }
}

/// An image create info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageCreateInfo {
    /// The mipmap level of the image.
    pub mipmap_levels: MipmapLevel,

    /// The format of the image data.
    pub format: ImageFormat,

    /// The extent of the image (width and height)
    pub extent: Extent2D,

    /// The expected usage of the image. This allow the driver to optimize the image
    /// for the specified usage. Most functions will have a undefined behavior if
    /// the image is used in a way that is not specified here.
    pub usage: ImageUsage,
}

impl Default for ImageCreateInfo {
    fn default() -> Self {
        Self {
            mipmap_levels: MipmapLevel::None,
            format: ImageFormat::R8G8B8A8SRGB,
            extent: Extent2D {
                height: 0,
                width: 0,
            },
            usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
        }
    }
}

/// An image mipmap type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MipmapLevel {
    /// The image will have no mipmaps.
    None,

    /// The image will have mipmaps automatically generated by the engine.
    Auto,

    /// The image will have mipmaps provided by the user.
    Count(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MipmapFilter {
    Nearest,
    Linear,
}

impl From<MipmapFilter> for vk::Filter {
    fn from(value: MipmapFilter) -> Self {
        match value {
            MipmapFilter::Nearest => vk::Filter::NEAREST,
            MipmapFilter::Linear => vk::Filter::LINEAR,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MipmapMode {
    Nearest,
    Linear,
}

impl From<MipmapMode> for vk::SamplerMipmapMode {
    fn from(value: MipmapMode) -> Self {
        match value {
            MipmapMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            MipmapMode::Linear => vk::SamplerMipmapMode::LINEAR,
        }
    }
}

// An 2D extent
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent2D {
    pub height: u32,
    pub width: u32,
}

impl From<vk::Extent2D> for Extent2D {
    fn from(extent: vk::Extent2D) -> Self {
        Self {
            height: extent.height,
            width: extent.width,
        }
    }
}

impl From<Extent2D> for vk::Extent2D {
    fn from(extent: Extent2D) -> Self {
        Self {
            height: extent.height,
            width: extent.width,
        }
    }
}

// An 3D extent
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent3D {
    pub height: u32,
    pub width: u32,
    pub depth: u32,
}

impl From<vk::Extent3D> for Extent3D {
    fn from(extent: vk::Extent3D) -> Self {
        Self {
            height: extent.height,
            width: extent.width,
            depth: extent.depth,
        }
    }
}

impl From<Extent3D> for vk::Extent3D {
    fn from(extent: Extent3D) -> Self {
        Self {
            height: extent.height,
            width: extent.width,
            depth: extent.depth,
        }
    }
}
