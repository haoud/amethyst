use std::sync::Arc;
use vulkanalia::prelude::v1_2::*;

use crate::device::RenderDevice;

use super::MipmapMode;

/// An image sampler. An image sampler is used used by the vulkan implementation to
/// read image data and apply filtering and other transformations for the shader.
pub struct ImageSampler {
    device: Arc<RenderDevice>,
    inner: vk::Sampler,
}

impl ImageSampler {
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, info: ImageSamplerCreatInfo) -> Self {
        let create_info = vk::SamplerCreateInfo::builder()
            .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .unnormalized_coordinates(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .compare_enable(false)
            /* Anisotropy informations */
            .anisotropy_enable(false)
            .max_anisotropy(0.0)
            /* Mimap informations */
            .mipmap_mode(info.mip_map_mode.into())
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mip_lod_bias(info.min_lod_bias)
            .min_lod(info.min_lod)
            .max_lod(info.max_lod)
            .build();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_sampler(&create_info, None)
                .expect("Failed to create image sampler")
        };

        Self { device, inner }
    }

    #[must_use]
    pub(crate) fn inner(&self) -> vk::Sampler {
        self.inner
    }
}

impl Drop for ImageSampler {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_sampler(self.inner, None);
        }
    }
}

/// Image sampler creation info.
pub struct ImageSamplerCreatInfo {
    pub mip_map_mode: MipmapMode,
    pub min_lod_bias: f32,
    pub min_lod: f32,
    pub max_lod: f32,
}

impl Default for ImageSamplerCreatInfo {
    fn default() -> Self {
        Self {
            mip_map_mode: MipmapMode::Linear,
            min_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: 0.0,
        }
    }
}
