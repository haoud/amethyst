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

/// A surface color space. For now, only non linear sRGB is supported.
/// Other formats are too exotic to be a priority for now.
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
