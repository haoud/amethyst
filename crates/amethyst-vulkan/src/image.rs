use bitflags::bitflags;
pub use vulkanalia::prelude::v1_2::*;

use crate::buffer::subbuffer::SubBuffer;

/// An image
pub struct Image {
    memory: ImageMemory,
    inner: vk::Image,
}

impl Image {
    /// Create a new image.
    #[must_use]
    pub(crate) fn new(inner: vk::Image, memory: ImageMemory) -> Self {
        Self { inner, memory }
    }

    /// Return the inner vulkan image.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::Image {
        self.inner
    }
}

/// An image view.
pub struct ImageView {
    inner: vk::ImageView,
}

impl ImageView {
    /// Create a new image view.
    #[must_use]
    pub(crate) fn new(inner: vk::ImageView) -> Self {
        Self { inner }
    }

    /// Return the inner vulkan image view.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::ImageView {
        self.inner
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ImageAccess: u32 {
        const UNDEFINED = 0;
        const COLOR_ATTACHMENT_WRITE = 1 << 0;
    }
}

impl From<ImageAccess> for vk::AccessFlags {
    fn from(value: ImageAccess) -> Self {
        match value {
            ImageAccess::COLOR_ATTACHMENT_WRITE => vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ImageAccess::UNDEFINED => vk::AccessFlags::empty(),
            _ => vk::AccessFlags::empty(),
        }
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
}

impl From<ImageLayout> for vk::ImageLayout {
    fn from(value: ImageLayout) -> Self {
        match value {
            ImageLayout::ColorAttachmentOptimal => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            ImageLayout::AttachmentOptimal => vk::ImageLayout::ATTACHMENT_OPTIMAL,
            ImageLayout::PresentSrcKhr => vk::ImageLayout::PRESENT_SRC_KHR,
            ImageLayout::Undefined => vk::ImageLayout::UNDEFINED,
        }
    }
}

pub enum ImageMemory {
    /// The image is backed by host or device memory.
    Buffer(SubBuffer<u8>),

    /// The image is backed by a swapchain.
    Swapchain,
}
