use crate::{
    buffer::subbuffer::SubBuffer,
    device::RenderDevice,
    prelude::{ImageDescriptorInfo, Pipeline},
    shader::ShaderStages,
};
use std::sync::Arc;
use vulkanalia::prelude::v1_2::*;

/// A descriptor pool. This is used to allocate descriptor sets.
pub struct DescriptorPool {
    device: Arc<RenderDevice>,
    inner: vk::DescriptorPool,
}

impl DescriptorPool {
    /// Create a new descriptor pool.
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, infos: &[DescriptorPoolCreateInfo]) -> Self {
        let pool_sizes = infos
            .iter()
            .map(|info| {
                vk::DescriptorPoolSize::builder()
                    .descriptor_count(info.descriptor_count)
                    .type_(info.descriptor_type.into())
                    .build()
            })
            .collect::<Vec<_>>();

        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(1)
            .build();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_descriptor_pool(&create_info, None)
                .expect("Failed to create descriptor pool")
        };

        Self { device, inner }
    }

    pub fn inner(&self) -> vk::DescriptorPool {
        self.inner
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_descriptor_pool(self.inner, None);
        }
    }
}

/// A descriptor set. This is a handle created from a descriptor pool and
/// descriptor set layout that can be used to use uniform variables in a
/// shader.
pub struct DescriptorSet {
    device: Arc<RenderDevice>,
    inner: vk::DescriptorSet,
    pool: Arc<DescriptorPool>,
}

impl DescriptorSet {
    /// Create a new descriptor set that will be configured using the given pipeline and
    /// allocated from the given descriptor pool.
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, pool: Arc<DescriptorPool>, pipeline: &Pipeline) -> Self {
        let layouts = pipeline
            .descriptor_set_layouts()
            .iter()
            .map(|layout| layout.inner())
            .collect::<Vec<_>>();

        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool.inner())
            .set_layouts(&layouts)
            .build();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .allocate_descriptor_sets(&allocate_info)
                .expect("Failed to allocate descriptor sets")[0]
        };

        Self {
            device,
            pool,
            inner,
        }
    }

    /// Update the descriptor set with the given buffer. The buffer must have
    /// exactly the same size as the uniform variable.
    pub fn update_buffer<T>(&self, binding: u32, buffer: &SubBuffer<T>) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .range(buffer.size() as u64)
            .buffer(buffer.inner())
            .offset(0)
            .build();

        let buffer_infos = &[buffer_info];
        let descriptor_write = vk::WriteDescriptorSet::builder()
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(buffer_infos)
            .dst_binding(binding)
            .dst_set(self.inner)
            .build();

        unsafe {
            self.device
                .logical()
                .inner()
                .update_descriptor_sets(&[descriptor_write], &[] as &[vk::CopyDescriptorSet]);
        }
    }

    /// Update the descriptor set with the given image view and sampler.
    pub fn update_image(&self, binding: u32, info: ImageDescriptorInfo) {
        let image_info = vk::DescriptorImageInfo::builder()
            .image_layout(info.layout.into())
            .image_view(info.view.inner())
            .sampler(info.sampler.inner())
            .build();

        let images_infos = &[image_info];
        let descriptor_write = vk::WriteDescriptorSet::builder()
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(images_infos)
            .dst_binding(binding)
            .dst_set(self.inner)
            .build();

        unsafe {
            self.device
                .logical()
                .inner()
                .update_descriptor_sets(&[descriptor_write], &[] as &[vk::CopyDescriptorSet]);
        }
    }

    /// Returns the raw Vulkan handle of the descriptor set.
    pub(crate) fn inner(&self) -> vk::DescriptorSet {
        self.inner
    }
}

/// A descriptor set layout. This is used to describe the uniform variables
/// layout used in a shader.
pub struct DescriptorSetLayout {
    device: Arc<RenderDevice>,
    inner: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    /// Create a new descriptor set layout. A descriptor set layout can contain
    /// multiple bindings, each binding describing a uniform variable.
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, layouts: &[DescriptorSetLayoutBinding]) -> Self {
        let bindings = layouts
            .iter()
            .map(|layout| {
                vk::DescriptorSetLayoutBinding::builder()
                    .descriptor_type(layout.descriptor_type.into())
                    .stage_flags(layout.shader_stages.into())
                    .binding(layout.binding)
                    .descriptor_count(1)
                    .build()
            })
            .collect::<Vec<_>>();

        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .build();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_descriptor_set_layout(&create_info, None)
                .expect("Failed to create descriptor set layout")
        };

        Self { device, inner }
    }

    /// Returns the raw Vulkan handle of the descriptor set layout.
    pub(crate) fn inner(&self) -> vk::DescriptorSetLayout {
        self.inner
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_descriptor_set_layout(self.inner, None);
        }
    }
}

/// Configuration used to create a descriptor set layout.
pub struct DescriptorSetLayoutBinding {
    /// The type of descriptor
    pub descriptor_type: DescriptorType,

    /// A set of shader stages flags to describe in which shader stages the
    /// descriptor is used. This allow some optimizations to be made by the
    /// driver if the descriptor is not used in some stages.
    pub shader_stages: ShaderStages,

    /// The binding number of this descriptor set layout.
    pub binding: u32,
}

impl Default for DescriptorSetLayoutBinding {
    fn default() -> Self {
        Self {
            descriptor_type: DescriptorType::Uniform,
            shader_stages: ShaderStages::ALL,
            binding: 0,
        }
    }
}

/// Configuration used to create a descriptor pool.
pub struct DescriptorPoolCreateInfo {
    /// The type of descriptor to allocate.
    pub descriptor_type: DescriptorType,

    /// The number of descriptors of the specified type to allocate. If more
    /// descriptors are allocated than the pool can handle, the allocation will
    /// fail.
    pub descriptor_count: u32,
}

impl Default for DescriptorPoolCreateInfo {
    fn default() -> Self {
        Self {
            descriptor_type: DescriptorType::Uniform,
            descriptor_count: 1,
        }
    }
}

/// The type of a descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorType {
    /// A sampler descriptor. This is used to sample textures in shaders.
    Sampler,

    /// A uniform descriptor. This is used to pass uniform variables to shaders,
    /// such as matrices, vectors, floats, etc.
    Uniform,
}

impl From<DescriptorType> for vk::DescriptorType {
    fn from(descriptor_type: DescriptorType) -> Self {
        match descriptor_type {
            DescriptorType::Sampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            DescriptorType::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
        }
    }
}
