use crate::{
    device::RenderDevice,
    image::surface::ColorSpace,
    prelude::{Extent2D, Image, ImageFormat, ImageMemory, ImageView, ImageViewCreateInfo},
    sync::Semaphore,
    Vulkan,
};
use amethyst_window::Window;
use std::{collections::HashSet, sync::Arc};
use vulkanalia::{
    prelude::v1_2::*,
    vk::{KhrSurfaceExtension, KhrSwapchainExtension},
};

/// A swapchain. A swapchain provides the ability to present rendering
/// results to a surface.
pub struct Swapchain {
    device: Arc<RenderDevice>,

    present_mode: PresentMode,
    format: SwapchainFormat,
    extent: Extent2D,

    images_views: Vec<ImageView>,
    images: Vec<Image>,
    inner: vk::SwapchainKHR,
}

impl Swapchain {
    /// Create a new swapchain from a window and a logical device. The swapchain
    /// will be created with the parameters specified in `info`. If the parameters
    /// are not supported by the device or invalid, the swapchain will be created
    /// with default parameters. Ideally, the swapchain capabilities should be
    /// queried before creating a swapchain to avoid this.
    #[must_use]
    pub fn new(window: &Window, device: Arc<RenderDevice>, info: SwapchainCreatInfo) -> Self {
        let support = SwapchainSupport::query(&device.vulkan(), device.physical().inner());

        let swapchain_format = support.validate_swapchain_format(SwapchainFormat {
            color_space: info.color_space,
            format: info.format,
        });
        let present_mode = support.validate_present_mode(info.present_mode);
        let images_count = support.validate_images_count(info.images_count);
        let extent = support.validate_extent(window, info.extent);
        let sharing_mode = info.sharing_mode;

        let queue_family_indices = &[];

        let swapchain_info = vk::SwapchainCreateInfoKHR::builder()
            .image_color_space(swapchain_format.color_space.into())
            .image_format(swapchain_format.format.into())
            .image_extent(vk::Extent2D::from(extent))
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode.into())
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .queue_family_indices(queue_family_indices)
            .pre_transform(support.capabilities.current_transform)
            .old_swapchain(vk::SwapchainKHR::null())
            .surface(device.vulkan().surface().inner())
            .present_mode(present_mode.into())
            .min_image_count(images_count)
            .image_array_layers(1)
            .clipped(true)
            .build();

        // Create the swapchain.
        let swapchain = unsafe {
            device
                .logical()
                .inner()
                .create_swapchain_khr(&swapchain_info, None)
                .expect("Failed to create swapchain")
        };

        // Get the swapchain images.
        let images = unsafe {
            device
                .logical()
                .inner()
                .get_swapchain_images_khr(swapchain)
                .expect("Failed to get swapchain images")
                .into_iter()
                .map(|image| Image::raw(image, ImageMemory::Swapchain))
                .collect::<Vec<_>>()
        };

        // Create an image views for each swapchain images.
        let images_views = images
            .iter()
            .map(|image| {
                let format: vk::Format = swapchain_format.format.into();
                ImageView::new(
                    Arc::clone(&device),
                    image,
                    ImageViewCreateInfo {
                        format: format.into(),
                        ..Default::default()
                    },
                )
            })
            .collect::<Vec<_>>();

        Self {
            inner: swapchain,
            images_views,
            present_mode,
            format: swapchain_format,
            extent,
            device,
            images,
        }
    }

    /// Acquire an image from the swapchain, and return its image index. The
    /// index can be used to retrieve the image/image view from the swapchain
    /// images/images views using the `images()` method.
    #[must_use]
    pub fn acquire_image_index(&self, semaphore: &Semaphore) -> u32 {
        unsafe {
            self.device
                .logical()
                .inner()
                .acquire_next_image_khr(
                    self.inner,
                    std::u64::MAX,
                    semaphore.inner(),
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next image")
                .0
        }
    }

    /// Present an image to the swapchain and wait for the present to complete.
    /// Before presenting the image, the `wait_semaphore` is waited on. After
    /// presenting the image, this function will wait for the present to complete.
    pub fn present_image(&self, info: SwapchainPresentInfo) {
        let wait_semaphores = &[info.wait_semaphore.inner()];
        let image_indices = &[info.image_index];
        let swapchains = &[self.inner];

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(wait_semaphores)
            .image_indices(image_indices)
            .swapchains(swapchains);

        unsafe {
            self.device
                .logical()
                .inner()
                .queue_present_khr(self.device.present_queue().inner(), &present_info)
                .expect("Failed to present image");

            self.device
                .logical()
                .inner()
                .queue_wait_idle(self.device.present_queue().inner())
                .expect("Failed to wait idle");
        }
    }

    /// Returns the present mode of the swapchain.
    #[must_use]
    pub fn present_mode(&self) -> PresentMode {
        self.present_mode
    }

    /// Returns the format of the swapchain.    
    #[must_use]
    pub fn format(&self) -> SwapchainFormat {
        self.format
    }

    /// Returns the extent of the swapchain.
    #[must_use]
    pub fn extent(&self) -> Extent2D {
        self.extent
    }

    /// Returns the swapchain image views.
    #[must_use]
    pub fn images_views(&self) -> &[ImageView] {
        &self.images_views
    }

    /// Returns the swapchain images.
    #[must_use]
    pub fn images(&self) -> &[Image] {
        &self.images
    }

    /// Returns the inner swapchain handle.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::SwapchainKHR {
        self.inner
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_swapchain_khr(self.inner, None);
        }
    }
}

/// Information required to create a swapchain.
pub struct SwapchainCreatInfo {
    /// The present mode to use for the swapchain. If not specified or not
    /// supported, the swapchain will be created with the best supported
    /// present mode.
    pub present_mode: PresentMode,

    /// The sharing mode to use for the swapchain.
    pub sharing_mode: SharingMode,

    /// The number of images to create for the swapchain. If not specified or
    /// not supported, the swapchain will be created with the recommended
    /// number of images.
    pub images_count: u32,

    /// The color space of the swapchain. If not specified or not supported,
    /// the swapchain will be created with the best supported color space.
    pub color_space: ColorSpace,

    /// The extent of the swapchain. If not specified or not supported, the
    /// swapchain will be created with the same extent as the window, if
    /// supported. Otherwise, the swapchain will be created with the best
    /// supported extent.
    pub extent: Option<Extent2D>,

    /// The format of the swapchain. If not specified or not supported, the
    /// swapchain will be created with the best supported format.
    pub format: ImageFormat,
}

impl Default for SwapchainCreatInfo {
    fn default() -> Self {
        Self {
            sharing_mode: SharingMode::Exclusive,
            present_mode: PresentMode::default(),
            color_space: ColorSpace::SrgbNonLinear,
            format: ImageFormat::R8G8B8A8SRGB,
            images_count: 1,
            extent: None,
        }
    }
}

/// The swapchain support capabilities. It contains the surface capabilities,
/// and all supported present modes and swapchain formats.
pub struct SwapchainSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub present_modes: Vec<PresentMode>,
    pub formats: Vec<SwapchainFormat>,
}

impl SwapchainSupport {
    /// Query the swapchain support capabilities.
    pub fn query(vulkan: &Vulkan, device: vk::PhysicalDevice) -> Self {
        let surface = vulkan.surface().inner();

        let capabilities = unsafe {
            vulkan
                .instance()
                .get_physical_device_surface_capabilities_khr(device, surface)
                .expect("Failed to query surface capabilities")
        };

        // Fetch the surface formats. Unsupported formats are filtered out.
        let formats = unsafe {
            vulkan
                .instance()
                .get_physical_device_surface_formats_khr(device, surface)
                .expect("Failed to query surface formats")
                .into_iter()
                .map(|format| SwapchainFormat::from(format))
                .filter(|format| format.format != ImageFormat::Undefined)
                .filter(|format| format.color_space != ColorSpace::Undefined)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        };

        let present_modes = unsafe {
            vulkan
                .instance()
                .get_physical_device_surface_present_modes_khr(device, surface)
                .expect("Failed to query surface present modes")
                .into_iter()
                .map(|mode| PresentMode::from(mode))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        };

        Self {
            capabilities,
            present_modes,
            formats,
        }
    }

    /// Validate the present mode. If the present mode is not supported or not
    /// specified, the FIFO present mode is returned.
    pub fn validate_present_mode(&self, present_mode: PresentMode) -> PresentMode {
        self.present_modes
            .iter()
            .map(|&mode| PresentMode::from(mode))
            .find(|&mode| mode == present_mode)
            .unwrap_or_default()
    }

    /// Validate the extent. If the extent is not supported or not specified,
    /// the current extent calculated from the surface capabilities and the
    /// window size is returned.
    pub fn validate_extent(&self, window: &Window, extent: Option<Extent2D>) -> Extent2D {
        if let Some(extent) = extent {
            if extent.width < self.capabilities.min_image_extent.width
                || extent.width > self.capabilities.max_image_extent.width
                || extent.height < self.capabilities.min_image_extent.height
                || extent.height > self.capabilities.max_image_extent.height
            {
                log::warn!(
                    "Requested extent {:?} is not supported, falling back to {:?}",
                    extent,
                    self.capabilities.current_extent
                );
                self.compute_window_extent(window)
            } else {
                extent
            }
        } else {
            self.compute_window_extent(window)
        }
    }

    /// Validate the swapchain format. If the format is not supported or not
    /// specified, the first supported format is returned.
    pub fn validate_swapchain_format(&self, format: SwapchainFormat) -> SwapchainFormat {
        self.formats
            .iter()
            .find(|&&f| f == format)
            .copied()
            .unwrap_or_else(|| {
                log::warn!(
                    "Requested format {:?} is not supported, falling back to {:?}",
                    format,
                    self.formats[0]
                );
                self.formats[0]
            })
    }

    /// Validate the image count. If the image count is not supported or not specified,
    /// the recommended image count calculated from the surface capabilities is returned.
    pub fn validate_images_count(&self, images: u32) -> u32 {
        if images < self.capabilities.min_image_count || images > self.capabilities.max_image_count
        {
            log::warn!(
                "Requested images count {} is not supported, falling back to {}",
                images,
                self.capabilities.min_image_count
            );
            self.best_image_count()
        } else {
            images
        }
    }

    /// Returns the best image format, if available, or the first one otherwise.
    pub fn best_format(&self) -> SwapchainFormat {
        self.formats
            .iter()
            .find(|format| {
                format.format == ImageFormat::R8G8B8A8SRGB
                    && format.color_space == ColorSpace::SrgbNonLinear
            })
            .copied()
            .unwrap_or_else(|| self.best_format())
    }

    /// Returns the best present mode, if available, or `FIFO` otherwise.
    pub fn best_present_mode(&self) -> PresentMode {
        self.present_modes
            .iter()
            .find(|&&mode| mode == PresentMode::Mailbox)
            .copied()
            .unwrap_or_else(|| PresentMode::Vsync)
    }

    /// Compute the best swapchain image count based on the swapchain
    /// capabilities.
    pub fn best_image_count(&self) -> u32 {
        let mut count = self.capabilities.min_image_count + 1;

        if self.capabilities.max_image_count != 0 && count > self.capabilities.max_image_count {
            count = self.capabilities.max_image_count;
        }
        count
    }

    /// Create an extent that match the window size and the swapchain capabilities. If
    /// the window size is not supported, the extent will be clamped to the best
    /// supported extent.
    pub fn compute_window_extent(&self, window: &Window) -> Extent2D {
        if self.capabilities.current_extent.width != u32::max_value() {
            Extent2D::from(self.capabilities.current_extent)
        } else {
            let height = window.inner().inner_size().height;
            let width = window.inner().inner_size().width;

            Extent2D {
                width: width.clamp(
                    self.capabilities.min_image_extent.width,
                    self.capabilities.max_image_extent.width,
                ),
                height: height.clamp(
                    self.capabilities.min_image_extent.height,
                    self.capabilities.max_image_extent.height,
                ),
            }
        }
    }

    /// Verify that the swapchain contains at least one format and one present mode
    /// that are supported.
    pub fn meet_requirements(&self) -> bool {
        !self.formats.is_empty() && !self.present_modes.is_empty()
    }
}

/// Information required to present an image to the swapchain.
pub struct SwapchainPresentInfo<'a> {
    /// The semaphore to wait on before presenting the image.
    pub wait_semaphore: &'a Semaphore,

    /// The index of the image to present. This index must be acquired from the
    /// swapchain before presenting the image.
    pub image_index: u32,
}

/// The present mode to use for the swapchain.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresentMode {
    /// The swapchain present images at the same rate as the display refresh rate.
    /// This mode guarantees that no tearing will happen and is supported by all
    /// Vulkan implementations.
    #[default]
    Vsync,

    /// The swapchain present images as fast as possible. This mode provides
    /// the best latency, but may result in tearing.
    Immediate,

    /// This mode is similar to `Vsync`, but the driver is allowed to present
    /// images if the vertical blanking period has already passed. This mode
    /// has a better latency than `Vsync` if the framerate is low, but may
    /// result in tearing.
    VsyncRelaxed,

    /// The presentation engine waits for the next vertical blanking period to
    /// submit the image to the display. This mode is similar to `Vsync`, as
    /// tearing cannot be observed, but the framerate is not limited to the
    /// display refresh rate.
    Mailbox,
}

impl From<vk::PresentModeKHR> for PresentMode {
    fn from(mode: vk::PresentModeKHR) -> Self {
        match mode {
            vk::PresentModeKHR::FIFO_RELAXED => Self::VsyncRelaxed,
            vk::PresentModeKHR::IMMEDIATE => Self::Immediate,
            vk::PresentModeKHR::MAILBOX => Self::Mailbox,
            vk::PresentModeKHR::FIFO => Self::Vsync,
            _ => Self::Vsync,
        }
    }
}

impl From<PresentMode> for vk::PresentModeKHR {
    fn from(mode: PresentMode) -> Self {
        match mode {
            PresentMode::VsyncRelaxed => vk::PresentModeKHR::FIFO_RELAXED,
            PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
            PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
            PresentMode::Vsync => vk::PresentModeKHR::FIFO,
        }
    }
}

/// The sharing mode to use for the swapchain. The exclusive mode should
/// be used if a ressource is exclusively used by one queue family at
/// a time, while the concurrent mode specifies that a ressource can
/// be used by multiple queue families at the same time.
pub enum SharingMode {
    Exclusive,
    Concurrent,
}

impl From<SharingMode> for vk::SharingMode {
    fn from(mode: SharingMode) -> Self {
        match mode {
            SharingMode::Concurrent => vk::SharingMode::CONCURRENT,
            SharingMode::Exclusive => vk::SharingMode::EXCLUSIVE,
        }
    }
}

impl From<vk::SharingMode> for SharingMode {
    fn from(mode: vk::SharingMode) -> Self {
        match mode {
            vk::SharingMode::CONCURRENT => Self::Concurrent,
            vk::SharingMode::EXCLUSIVE => Self::Exclusive,
            _ => panic!("Invalid sharing mode"),
        }
    }
}

/// A swapchain format. This is simply the combination of a color space
/// and a surface format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SwapchainFormat {
    pub color_space: ColorSpace,
    pub format: ImageFormat,
}

impl From<vk::SurfaceFormatKHR> for SwapchainFormat {
    fn from(format: vk::SurfaceFormatKHR) -> Self {
        Self {
            color_space: format.color_space.into(),
            format: format.format.into(),
        }
    }
}

impl From<SwapchainFormat> for vk::SurfaceFormatKHR {
    fn from(format: SwapchainFormat) -> Self {
        Self {
            color_space: format.color_space.into(),
            format: format.format.into(),
        }
    }
}
