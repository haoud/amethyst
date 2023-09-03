use crate::{device::RenderDevice, surface::Format};
use std::{mem::ManuallyDrop, sync::Arc};
use vk_mem_vulkanalia::{Allocation, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};
use vulkanalia::prelude::v1_2::*;

pub mod allocator;
pub mod subbuffer;

/// A buffer. This contains a Vulkan buffer and the memory allocation for the
/// buffer.
pub struct Buffer {
    allocation: ManuallyDrop<Allocation>,
    device: Arc<RenderDevice>,
    inner: vk::Buffer,
    kind: BufferKind,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            let allocation = ManuallyDrop::take(&mut self.allocation);
            self.device
                .buffer_allocator()
                .inner()
                .destroy_buffer(self.inner, allocation);
        }
    }
}

/// The kind of buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferKind {
    Uniforms,
    Vertices,
    Indices,
    Storage,
}

impl From<BufferKind> for vk::BufferUsageFlags {
    fn from(kind: BufferKind) -> Self {
        match kind {
            BufferKind::Uniforms => vk::BufferUsageFlags::UNIFORM_BUFFER,
            BufferKind::Vertices => vk::BufferUsageFlags::VERTEX_BUFFER,
            BufferKind::Storage => vk::BufferUsageFlags::STORAGE_BUFFER,
            BufferKind::Indices => vk::BufferUsageFlags::INDEX_BUFFER,
        }
    }
}

/// Location where the buffer should be allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferMemoryLocation {
    /// Prefer device local memory. This is usually the fastest memory type, but not accessible
    /// by the CPU. Copying data to this memory type requires a staging buffer. Some GPUs may not
    /// have device local memory (e.g. integrated GPUs), in which case host visible memory will
    /// be used instead.
    PreferDeviceLocal,

    /// Prefer host visible memory. This is usually slower than device local memory, but is
    /// directly accessible by the CPU. This is useful for buffers that are updated frequently,
    /// to avoid saturating the PCIe bus of the GPU.
    PreferHostVisible,
}

impl From<BufferMemoryLocation> for AllocationCreateInfo {
    fn from(location: BufferMemoryLocation) -> Self {
        match location {
            BufferMemoryLocation::PreferHostVisible => Self {
                usage: MemoryUsage::AutoPreferHost,
                flags: AllocationCreateFlags::MAPPED,
                ..Default::default()
            },
            BufferMemoryLocation::PreferDeviceLocal => Self {
                usage: MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
        }
    }
}

///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferTransfert {
    /// The buffer can be used as a source for a transfer operation.
    Destination,

    /// The buffer can be used as a destination for a transfer operation.
    Source,

    /// The buffer can be used as a source and destination for a transfer
    /// operation.
    All,
}

impl From<BufferTransfert> for vk::BufferUsageFlags {
    fn from(usage: BufferTransfert) -> Self {
        match usage {
            BufferTransfert::Destination => vk::BufferUsageFlags::TRANSFER_DST,
            BufferTransfert::Source => vk::BufferUsageFlags::TRANSFER_SRC,
            BufferTransfert::All => {
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC
            }
        }
    }
}

/// The access mode of the buffer. This allows the buffer allocator to optimize
/// the memory allocation location. Violating the access mode may not result in
/// an error, but may result in a huge performance penalty in some cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferAccessMode {
    /// The buffer will not be accessed by the CPU.
    None,

    /// The buffer will be accessed by the CPU in a sequential manner, for example
    /// by used `std::ptr::copy_nonoverlapping` to copy data to the buffer. Watch out
    /// for implicit reads introduced by writing to a buffer, for example when
    /// incrementing or decrementing a value in a buffer.
    Sequential,

    /// The buffer will be accessed by the CPU in a random manner. The memory
    /// allocated can be read and written to at any order.
    Random,
}

impl From<BufferAccessMode> for AllocationCreateFlags {
    fn from(mode: BufferAccessMode) -> Self {
        match mode {
            BufferAccessMode::Sequential => AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            BufferAccessMode::Random => AllocationCreateFlags::HOST_ACCESS_RANDOM,
            BufferAccessMode::None => AllocationCreateFlags::empty(),
        }
    }
}

pub unsafe trait VertexBindingDescription {
    fn binding_description() -> BindingDescription;
}

pub unsafe trait VertexAttributeDescription {
    fn attribute_descriptions() -> Vec<AttributeDescription>;
}

/// A binding description.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindingDescription {
    /// The binding number that this structure describes.
    pub binding: u32,

    /// The byte stride between consecutive elements within the buffer.
    pub stride: u32,
}

impl From<BindingDescription> for vk::VertexInputBindingDescription {
    fn from(binding: BindingDescription) -> Self {
        vk::VertexInputBindingDescription {
            input_rate: vk::VertexInputRate::VERTEX,
            binding: binding.binding,
            stride: binding.stride,
        }
    }
}

/// An attribute description. This is used to describe how the pipeline should
/// interpret the vertex buffer data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeDescription {
    /// The format of the vertex attribute data. This is the size and type of
    /// the vertex attribute data.
    pub format: Format,

    /// The shader input location number for this attribute.
    pub location: u32,

    // The binding number which this attribute takes its data from.
    pub binding: u32,

    /// A byte offset of this attribute relative to the start of an element in the
    /// vertex input binding.
    pub offset: u32,
}

impl From<AttributeDescription> for vk::VertexInputAttributeDescription {
    fn from(attribute: AttributeDescription) -> Self {
        vk::VertexInputAttributeDescription {
            format: attribute.format.into(),
            location: attribute.location,
            binding: attribute.binding,
            offset: attribute.offset,
        }
    }
}
