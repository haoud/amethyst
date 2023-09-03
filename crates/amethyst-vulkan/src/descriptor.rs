use crate::{
    buffer::subbuffer::SubBuffer, device::RenderDevice, prelude::Pipeline, shader::ShaderStages,
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
    pub fn new(device: Arc<RenderDevice>, info: DescriptorPoolCreateInfo) -> Self {
        let pool_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(info.descriptor_count)
            .build();

        let pool_sizes = &[pool_size];
        let create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes)
            .max_sets(info.max_sets)
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
    /// the same size as the uniform variable.
    pub fn update<T>(&self, buffer: &SubBuffer<T>) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .range(buffer.size() as u64)
            .buffer(buffer.inner())
            .offset(0)
            .build();

        let buffer_infos = &[buffer_info];
        let descriptor_write = vk::WriteDescriptorSet::builder()
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(buffer_infos)
            .dst_set(self.inner)
            .dst_binding(0)
            .build();

        unsafe {
            self.device
                .logical()
                .inner()
                .update_descriptor_sets(&[descriptor_write], &[] as &[vk::CopyDescriptorSet]);
        }
    }

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
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
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
            shader_stages: ShaderStages::ALL,
            binding: 0,
        }
    }
}

/// Configuration used to create a descriptor pool.
pub struct DescriptorPoolCreateInfo {
    /// The number of descriptors of the specified type to allocate.
    pub descriptor_count: u32,

    /// The maximum number of sets that can be allocated from the pool.
    pub max_sets: u32,
}

impl Default for DescriptorPoolCreateInfo {
    fn default() -> Self {
        Self {
            descriptor_count: 1,
            max_sets: 1,
        }
    }
}
