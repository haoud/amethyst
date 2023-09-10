use amethyst_window::Window;
use std::sync::Arc;
use vulkanalia::{prelude::v1_2::*, vk::KhrSurfaceExtension};

/// A surface.
pub struct Surface {
    instance: Arc<Instance>,
    inner: vk::SurfaceKHR,
}

impl Surface {
    /// Create a new surface from a window.
    #[must_use]
    pub(crate) fn from_window(instance: Arc<Instance>, window: &Window) -> Self {
        let inner = unsafe {
            let window = window.inner();
            vulkanalia::window::create_surface(&instance, window, window)
                .expect("Failed to create surface")
        };

        Self { instance, inner }
    }

    /// Return the inner vulkan surface.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::SurfaceKHR {
        self.inner
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.instance
                .destroy_surface_khr(self.inner, None);
        }
    }
}

/// A surface format.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    #[default]
    Undefined,
    B8G8R8A8Srgb,
    R8G8B8A8Srgb,
    R32Sfloat,
    R32G32Sfloat,
    R32G32B32Sfloat,
    R32G32B32A32Sfloat,
    D32Sfloat,
}

impl From<Format> for vk::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::Undefined => vk::Format::UNDEFINED,
            Format::R32Sfloat => vk::Format::R32_SFLOAT,
            Format::R32G32Sfloat => vk::Format::R32G32_SFLOAT,
            Format::R32G32B32Sfloat => vk::Format::R32G32B32_SFLOAT,
            Format::R32G32B32A32Sfloat => vk::Format::R32G32B32A32_SFLOAT,
            Format::B8G8R8A8Srgb => vk::Format::B8G8R8A8_SRGB,
            Format::R8G8B8A8Srgb => vk::Format::R8G8B8A8_SRGB,
            Format::D32Sfloat => vk::Format::D32_SFLOAT,
        }
    }
}

impl From<vk::Format> for Format {
    fn from(format: vk::Format) -> Self {
        match format {
            vk::Format::UNDEFINED => Self::Undefined,
            vk::Format::R32_SFLOAT => Self::R32Sfloat,
            vk::Format::R32G32_SFLOAT => Self::R32G32Sfloat,
            vk::Format::R32G32B32_SFLOAT => Self::R32G32B32Sfloat,
            vk::Format::R32G32B32A32_SFLOAT => Self::R32G32B32A32Sfloat,
            vk::Format::R8G8B8A8_SRGB => Self::R8G8B8A8Srgb,
            vk::Format::B8G8R8A8_SRGB => Self::B8G8R8A8Srgb,
            vk::Format::D32_SFLOAT => Self::D32Sfloat,
            _ => Self::Undefined,
        }
    }
}

/// A surface color space.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    #[default]
    Undefined,
    SrgbNonLinear,
}

impl From<vk::ColorSpaceKHR> for ColorSpace {
    fn from(color_space: vk::ColorSpaceKHR) -> Self {
        match color_space {
            vk::ColorSpaceKHR::SRGB_NONLINEAR => Self::SrgbNonLinear,
            _ => Self::Undefined,
        }
    }
}

impl From<ColorSpace> for vk::ColorSpaceKHR {
    fn from(color_space: ColorSpace) -> Self {
        match color_space {
            ColorSpace::SrgbNonLinear => vk::ColorSpaceKHR::SRGB_NONLINEAR,
            _ => vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }
    }
}

// An 2D extent
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
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
    pub width: u32,
    pub height: u32,
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
