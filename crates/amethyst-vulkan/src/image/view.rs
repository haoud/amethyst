use super::{Image, ImageFormat, ImageSubResourceRange};
use crate::device::RenderDevice;
use std::sync::Arc;
pub use vulkanalia::prelude::v1_2::*;

/// An image view. Image objects are not directly accessed by pipeline shaders
/// for reading or writing image data. Instead, image views representing contiguous
/// ranges of the image subresources and containing additional metadata are used
/// for that purpose. Views must be created on images of compatible types, and must
/// represent a valid subset of image subresources.
pub struct ImageView {
    device: Arc<RenderDevice>,
    inner: vk::ImageView,
}

impl ImageView {
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, image: &Image, info: ImageViewCreateInfo) -> Self {
        let components = vk::ComponentMapping::builder()
            .r(vk::ComponentSwizzle::IDENTITY)
            .g(vk::ComponentSwizzle::IDENTITY)
            .b(vk::ComponentSwizzle::IDENTITY)
            .a(vk::ComponentSwizzle::IDENTITY);

        let create_info = vk::ImageViewCreateInfo::builder()
            .subresource_range(vk::ImageSubresourceRange::from(info.subresource))
            .view_type(info.kind.into())
            .format(info.format.into())
            .components(components)
            .image(image.inner())
            .build();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_image_view(&create_info, None)
                .expect("Failed to create image view")
        };

        Self { device, inner }
    }

    /// Return the inner vulkan image view.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::ImageView {
        self.inner
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_image_view(self.inner, None);
        }
    }
}

/// Image view creation info.
pub struct ImageViewCreateInfo {
    /// The image subresource range. This describes the image subresources that
    /// can be accessed through this image view. By default, this is set to
    /// include all subresources of the image.
    pub subresource: ImageSubResourceRange,

    /// The format of the image data. This must be compatible with the format
    /// specified when the image was created.
    pub format: ImageFormat,

    /// The type of the image view. This must be compatible with the image.
    /// Usually, the ImageViewKind should be `Type2D`, meaning that the image
    /// view is a 2D texture.
    pub kind: ImageViewKind,
}

impl Default for ImageViewCreateInfo {
    fn default() -> Self {
        Self {
            subresource: ImageSubResourceRange::default(),
            format: ImageFormat::R8G8B8A8SRGB,
            kind: ImageViewKind::Type2D,
        }
    }
}

/// An image view type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageViewKind {
    Type1D,
    Type2D,
    Type3D,
    TypeCube,
    Type1DArray,
    Type2DArray,
    TypeCubeArray,
}

impl From<ImageViewKind> for vk::ImageViewType {
    fn from(value: ImageViewKind) -> Self {
        match value {
            ImageViewKind::Type1D => vk::ImageViewType::_1D,
            ImageViewKind::Type2D => vk::ImageViewType::_2D,
            ImageViewKind::Type3D => vk::ImageViewType::_3D,
            ImageViewKind::TypeCube => vk::ImageViewType::CUBE,
            ImageViewKind::Type1DArray => vk::ImageViewType::_1D_ARRAY,
            ImageViewKind::Type2DArray => vk::ImageViewType::_2D_ARRAY,
            ImageViewKind::TypeCubeArray => vk::ImageViewType::CUBE_ARRAY,
        }
    }
}
