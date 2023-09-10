use std::sync::Arc;
use vulkanalia::prelude::v1_2::*;

use crate::device::RenderDevice;

pub struct ImageSampler {
    device: Arc<RenderDevice>,
    inner: vk::Sampler,
}

impl ImageSampler {
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, _info: ImageSamplerCreatInfo) -> Self {
        let create_info = vk::SamplerCreateInfo::builder()
            .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .compare_op(vk::CompareOp::ALWAYS)
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .unnormalized_coordinates(false)
            .anisotropy_enable(false)
            .compare_enable(false)
            .max_anisotropy(0.0)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0)
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

pub struct ImageSamplerCreatInfo {}

impl Default for ImageSamplerCreatInfo {
    fn default() -> Self {
        Self {}
    }
}
