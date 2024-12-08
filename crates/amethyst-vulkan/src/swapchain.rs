use crate::{context::VulkanContext, device::VulkanDevice, semaphore::Semaphore};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;
use vk::{KhrSurfaceExtension, KhrSwapchainExtension};
use vulkanalia::prelude::v1_3::*;

/// A Vulkan surface that can be used to present images to a window when the rendering
/// is done. This can be considered as a pointer to the window contents, allowing the
/// rendering to be displayed on the screen.
#[derive(Debug)]
pub struct Surface {
    context: Arc<VulkanContext>,
    surface: vk::SurfaceKHR,
}

impl Surface {
    #[must_use]
    pub fn new<T: HasDisplayHandle + HasWindowHandle>(
        context: Arc<VulkanContext>,
        handle: T,
    ) -> Self {
        let surface = unsafe {
            vulkanalia::window::create_surface(context.instance(), &handle, &handle)
                .expect("Failed to create surface")
        };

        Self { surface, context }
    }

    /// Returns the inner Vulkan surface object.
    #[must_use]
    pub const fn inner(&self) -> vk::SurfaceKHR {
        self.surface
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.context
                .instance()
                .destroy_surface_khr(self.surface, None);
        }
    }
}

/// A Vulkan swapchain that can be used to present images to a surface.
#[derive(Debug)]
pub struct VulkanSwapchain {
    /// The Vulkan device this swapchain is associated with.
    device: Arc<VulkanDevice>,

    /// The surface this swapchain is associated with.
    surface: Surface,

    /// Information about the swapchain, such as supported formats and present modes.
    support: VulkanSwapchainSupport,

    /// The format of the swapchain images.
    format: vk::Format,

    /// The extent of the swapchain images.
    extent: vk::Extent2D,

    /// The present mode of the swapchain.
    present_mode: vk::PresentModeKHR,

    /// The swapchain images.
    images: Vec<vk::Image>,

    /// The image views of the swapchain images.
    views: Vec<vk::ImageView>,

    /// The swapchain vulkan objects.
    inner: vk::SwapchainKHR,
}

impl VulkanSwapchain {
    #[must_use]
    pub fn new(context: Arc<VulkanContext>, device: Arc<VulkanDevice>, surface: Surface) -> Self {
        let support = VulkanSwapchainSupport::new(&context, &device, &surface);

        // Choose the swapchain present mode. By default, we use the FIFO present mode as it is
        // guaranteed to be supported by all devices that support the swapchain extension.
        let present_mode = vk::PresentModeKHR::FIFO;

        // Choose the swapchain extent. This is the resolution of the swapchain images. By default,
        // we use the current extent of the surface provided by the surface capabilities.
        let extent = unsafe {
            context
                .instance()
                .get_physical_device_surface_capabilities_khr(device.physical(), surface.inner())
                .expect("Failed to get physical device surface capabilities")
                .current_extent
        };

        // Choose the swapchain format. By default, we use the B8G8R8A8_SRGB format as it is
        // a common format that is supported by most devices with good color accuracy. If this
        // format is not supported, we fallback to the first supported format.
        let format = support
            .formats()
            .iter()
            .find(|f| f.format == vk::Format::B8G8R8A8_SRGB)
            .map(|f| f.format)
            .unwrap_or_else(|| {
                support
                    .formats()
                    .first()
                    .expect("No supported formats found")
                    .format
            });

        // Choose the swapchain color space. By default, we use the SRGB_NONLINEAR color space as
        // it is a common color space that is supported by most devices with good color accuracy.
        // If this color space is not supported with the chosen format, we fallback to the first
        // supported color space that is compatible with the chosen format.
        let color_space = support
            .formats()
            .iter()
            .filter(|f| f.format == format)
            .find(|f| f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .map(|f| f.color_space)
            .unwrap_or_else(|| {
                support
                    .formats()
                    .iter()
                    .find(|f| f.format == format)
                    .expect("No supported formats found")
                    .color_space
            });

        // Get the queue family that are allowed to present to the surface.
        let queue_family_indices = [
            device.queues_info().main_family(),
            device.queues_info().present_family(),
        ];

        // Choose the sharing mode of the swapchain images. If the queue families are different,
        // we use the concurrent sharing mode. Otherwise, we use the exclusive sharing mode.
        // The concurrent sharing mode allows the images to be used across multiple queue families
        // without explicit ownership transfers. The exclusive sharing mode requires explicit
        // ownership transfers between queue families, but may offer better performance.
        let sharing_mode = if queue_family_indices[0] != queue_family_indices[1] {
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        };

        // Build the swapchain create info.
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(support.capabilities().current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .queue_family_indices(&queue_family_indices)
            .min_image_count(support.clamp_image_count(2))
            .image_sharing_mode(sharing_mode)
            .image_color_space(color_space)
            .image_format(format)
            .image_extent(extent)
            .image_array_layers(1)
            .present_mode(present_mode)
            .surface(surface.inner())
            .clipped(true);

        // Create the swapchain.
        let swapchain = unsafe {
            device
                .logical()
                .create_swapchain_khr(&swapchain_create_info, None)
                .expect("Failed to create swapchain")
        };

        // Get the swapchain images from the swapchain. Those images will has the
        // same format and extent as the swapchain, and will be used to be presented
        // to the surface.
        let images = unsafe {
            device
                .logical()
                .get_swapchain_images_khr(swapchain)
                .expect("Failed to get swapchain images")
        };

        // Create the image views for the swapchain images. An image view is quite literally a
        // view into an image. It describes how to access the image and which part of the image
        // to access, for example if it should be treated as a 2D texture depth texture without
        // any mipmapping levels.
        let views = images
            .iter()
            .map(|&image| {
                let components = vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                };

                let subresource_range = vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_array_layer: 0,
                    base_mip_level: 0,
                    level_count: 1,
                    layer_count: 1,
                };

                let view_create_info = vk::ImageViewCreateInfo::builder()
                    .subresource_range(subresource_range)
                    .view_type(vk::ImageViewType::_2D)
                    .components(components)
                    .format(format)
                    .image(image);

                unsafe {
                    device
                        .logical()
                        .create_image_view(&view_create_info, None)
                        .expect("Failed to create image view")
                }
            })
            .collect();

        Self {
            device,
            surface,
            support,
            format,
            extent,
            present_mode,
            images,
            views,
            inner: swapchain,
        }
    }

    /// Acquire an image from the swapchain, and return its image index. The
    /// index can be used to retrieve the image/image view from the swapchain
    /// images/images views using the `images()` method.
    /// If no image is available, this function will block indefinitely until
    /// an image is available.
    #[must_use]
    pub fn acquire_next_image_index(&self, semaphore: &Semaphore) -> u32 {
        unsafe {
            self.device
                .logical()
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

    /// Acquire an image from the swapchain, and return the image, its image view, and
    /// its index. The image and image view can be used to render to the image, and the
    /// index can be used to present the image to the surface.
    /// If no image is available, this function will block indefinitely until an image
    /// is available.
    #[must_use]
    pub fn acquire_next_image(&self, semaphore: &Semaphore) -> (u32, vk::Image, vk::ImageView) {
        let index = self.acquire_next_image_index(semaphore);
        let image = self.images[index as usize];
        let view = self.views[index as usize];
        (index, image, view)
    }

    /// Present an image to the surface. The image is identified by its index
    /// in the swapchain images, and the semaphore parameter allows the presentation
    /// to be synchronized with other operations.
    ///
    /// # Important
    /// This function returns immediately after the presentation is submitted to the
    /// queue. The actual presentation may not have been completed yet. To ensure that
    /// the presentation is completed, you can use a fence or a semaphore to wait for
    /// the presentation to be completed.
    pub fn present_image(&self, queue: vk::Queue, image_index: u32, wait: &Semaphore) {
        let wait_semaphores = [wait.inner()];
        let image_indices = [image_index];
        let swapchains = [self.inner];

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .image_indices(&image_indices)
            .swapchains(&swapchains);

        unsafe {
            self.device
                .logical()
                .queue_present_khr(queue, &present_info)
                .expect("Failed to present image");
        }
    }

    /// Returns the surface used to create the swapchain.s
    #[must_use]
    pub const fn surface(&self) -> &Surface {
        &self.surface
    }

    /// Returns the format of the swapchain images.
    #[must_use]
    pub const fn format(&self) -> vk::Format {
        self.format
    }

    /// Returns the extent of the swapchain images.
    #[must_use]
    pub const fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    /// Returns the present mode of the swapchain.
    #[must_use]
    pub const fn present_mode(&self) -> vk::PresentModeKHR {
        self.present_mode
    }

    /// Returns the supported capabilities, formats, and present modes of the swapchain.
    #[must_use]
    pub const fn support(&self) -> &VulkanSwapchainSupport {
        &self.support
    }

    /// Returns the swapchain images.
    #[must_use]
    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    /// Returns the image views of the swapchain images.
    pub fn image_views(&self) -> &[vk::ImageView] {
        &self.views
    }
}

impl Drop for VulkanSwapchain {
    fn drop(&mut self) {
        unsafe {
            for view in self.views.drain(..) {
                self.device.logical().destroy_image_view(view, None);
            }

            self.device
                .logical()
                .destroy_swapchain_khr(self.inner, None);
        }
    }
}

/// Information about the supported formats, present modes, and capabilities of a Vulkan swapchain.
/// This information can be used to create a swapchain with the best possible settings that are
/// supported by the device.
#[derive(Debug)]
pub struct VulkanSwapchainSupport {
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
    capabilities: vk::SurfaceCapabilitiesKHR,
}

impl VulkanSwapchainSupport {
    /// Creates a new `VulkanSwapchainInfo` object that contains the supported formats, present
    /// modes, and capabilities of the swapchain.
    #[must_use]
    pub fn new(context: &VulkanContext, device: &VulkanDevice, surface: &Surface) -> Self {
        let formats = unsafe {
            context
                .instance()
                .get_physical_device_surface_formats_khr(device.physical(), surface.inner())
                .expect("Failed to get physical device surface formats")
        };
        let present_modes = unsafe {
            context
                .instance()
                .get_physical_device_surface_present_modes_khr(device.physical(), surface.inner())
                .expect("Failed to get physical device surface present modes")
        };

        let capabilities = unsafe {
            context
                .instance()
                .get_physical_device_surface_capabilities_khr(device.physical(), surface.inner())
                .expect("Failed to get physical device surface capabilities")
        };

        Self {
            formats,
            present_modes,
            capabilities,
        }
    }

    /// Returns whether the swapchain supports the given present mode or not.
    #[must_use]
    pub fn support_present_mode(&self, present_mode: vk::PresentModeKHR) -> bool {
        self.present_modes.iter().any(|&p| p == present_mode)
    }

    /// Returns whether the swapchain supports the given format or not.
    #[must_use]
    pub fn support_format(&self, format: vk::Format) -> bool {
        self.formats.iter().any(|f| f.format == format)
    }

    /// Clamps the given image count to the supported range of the swapchain. This guarantees that
    /// the returned image count is within the supported range.
    #[must_use]
    pub fn clamp_image_count(&self, count: u32) -> u32 {
        let min = self.capabilities.min_image_count;
        let max = self.capabilities.max_image_count.max(min + 1);
        count.clamp(min, max)
    }

    /// Returns the supported capabilities of the swapchain.
    #[must_use]
    pub const fn capabilities(&self) -> vk::SurfaceCapabilitiesKHR {
        self.capabilities
    }

    /// Returns the supported formats of the swapchain.
    #[must_use]
    pub fn formats(&self) -> &[vk::SurfaceFormatKHR] {
        &self.formats
    }

    /// Returns the supported present modes of the swapchain.
    #[must_use]
    pub fn present_modes(&self) -> &[vk::PresentModeKHR] {
        &self.present_modes
    }
}
