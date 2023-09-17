use crate::{device::RenderDevice, prelude::ImageFormat};
use std::{mem::ManuallyDrop, sync::Arc};
use vk_mem_vulkanalia::{
    Alloc, Allocation, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage,
};
use vulkanalia::{prelude::v1_2::*, vk::BufferUsageFlags};

pub mod allocator;
pub mod subbuffer;

/// A buffer. This contains a Vulkan buffer and the memory allocation for the
/// buffer.
pub struct Buffer {
    allocation: ManuallyDrop<Allocation>,
    device: Arc<RenderDevice>,
    info: BufferCreateInfo,
    inner: vk::Buffer,
}

impl Buffer {
    #[must_use]
    pub fn empty(device: Arc<RenderDevice>, info: BufferCreateInfo) -> Self {
        let mut create_info = AllocationCreateInfo::from(info.usage.location);
        create_info.flags |= AllocationCreateFlags::from(info.usage.access);
        if info.usage.memory_type != 0 {
            create_info = AllocationCreateInfo {
                flags: AllocationCreateFlags::from(info.usage.access),
                memory_type_bits: info.usage.memory_type,
                ..Default::default()
            };
        }

        let (inner, allocation) = unsafe {
            let transfert = vk::BufferUsageFlags::from(info.usage.transfer);
            let flags = vk::BufferUsageFlags::from(info.kind);

            let buffer_create_info = vk::BufferCreateInfo::builder()
                .usage(transfert | flags)
                .size(info.size as u64)
                .build();

            device
                .buffer_allocator()
                .inner()
                .create_buffer_with_alignment(
                    &buffer_create_info,
                    &create_info,
                    info.alignment as u64,
                )
                .expect("Failed to create buffer")
        };

        Self {
            allocation: ManuallyDrop::new(allocation),
            device,
            inner,
            info,
        }
    }

    /// Return a u8 mutable slice to the mapped memory of the buffer. This is
    /// only valid if the buffer is host visible. If the buffer is not host
    /// visible, this will return `None`.
    pub fn as_u8_slice_mut(&mut self) -> Option<&mut [u8]> {
        self.as_u8_slice().map(|slice| unsafe {
            std::slice::from_raw_parts_mut(slice.as_ptr() as *mut u8, self.info.size)
        })
    }

    /// Return a u8 slice to the mapped memory of the buffer. This is only valid
    /// if the buffer is host visible. If the buffer is not host visible, this
    /// will return `None`.
    pub fn as_u8_slice(&self) -> Option<&[u8]> {
        if self.info.usage.location != BufferMemoryLocation::PreferHostVisible {
            return None;
        }

        let allocation_info = unsafe {
            self.device
                .buffer_allocator()
                .inner()
                .get_allocation_info(&self.allocation)
                .expect("Failed to get allocation info")
        };

        unsafe {
            Some(std::slice::from_raw_parts(
                allocation_info.mapped_data as *const u8,
                self.info.size,
            ))
        }
    }

    /// Returns the offset of the buffer in the device memory.
    pub(crate) fn device_memory_offset(&self) -> u64 {
        unsafe {
            self.device
                .buffer_allocator()
                .inner()
                .get_allocation_info(&self.allocation)
                .expect("Failed to get allocation info")
                .offset
        }
    }

    /// Returns the device memory of the buffer.
    pub(crate) fn device_memory(&self) -> vk::DeviceMemory {
        unsafe {
            self.device
                .buffer_allocator()
                .inner()
                .get_allocation_info(&self.allocation)
                .expect("Failed to get allocation info")
                .device_memory
        }
    }

    /// Returns the inner Vulkan buffer.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::Buffer {
        self.inner
    }

    /// Return the information used to create the buffer.
    pub fn info(&self) -> &BufferCreateInfo {
        &self.info
    }
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
    None,
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
            BufferKind::None => vk::BufferUsageFlags::empty(),
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
                required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
                preferred_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
                flags: AllocationCreateFlags::MAPPED,
                usage: MemoryUsage::AutoPreferHost,
                ..Default::default()
            },
            BufferMemoryLocation::PreferDeviceLocal => Self {
                usage: MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
        }
    }
}

/// The transfer mode of the buffer. This allows the buffer allocator to optimize
/// the memory allocation location. Violating the transfer mode may not result in
/// an error, but may result in a huge performance penalty in some cases.
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

impl From<BufferTransfert> for BufferUsageFlags {
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
    pub format: ImageFormat,

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

/// A buffer create info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BufferUsageInfo {
    pub location: BufferMemoryLocation,
    pub transfer: BufferTransfert,
    pub access: BufferAccessMode,
    pub memory_type: u32,
}

impl BufferUsageInfo {
    /// A pre-defined buffer create info for static mesh rendering. This buffer
    /// will be stored in device local memory and data will be copied to it once
    /// using a staging buffer.
    pub const STATIC_RENDERING: BufferUsageInfo = BufferUsageInfo {
        location: BufferMemoryLocation::PreferDeviceLocal,
        transfer: BufferTransfert::Destination,
        access: BufferAccessMode::None,
        memory_type: 0,
    };

    /// A pre-defined buffer create info for creating a staging buffer. This
    /// buffer will be stored in host visible memory and can be used to copy
    /// data to device local memory.
    pub const STAGING: BufferUsageInfo = BufferUsageInfo {
        location: BufferMemoryLocation::PreferHostVisible,
        transfer: BufferTransfert::Source,
        access: BufferAccessMode::Sequential,
        memory_type: 0,
    };

    /// A pre-defined buffer create info for creating a uniform buffer. This
    /// buffer will be stored in host visible memory and can be used to update
    /// the buffer data frequently.
    pub const UNIFORM: BufferUsageInfo = BufferUsageInfo {
        location: BufferMemoryLocation::PreferDeviceLocal,
        transfer: BufferTransfert::Destination,
        access: BufferAccessMode::Sequential,
        memory_type: 0,
    };

    pub const IMAGE: BufferUsageInfo = BufferUsageInfo {
        location: BufferMemoryLocation::PreferDeviceLocal,
        transfer: BufferTransfert::Destination,
        access: BufferAccessMode::Sequential,
        memory_type: 0,
    };
}

impl Default for BufferUsageInfo {
    fn default() -> Self {
        Self {
            location: BufferMemoryLocation::PreferDeviceLocal,
            transfer: BufferTransfert::Destination,
            access: BufferAccessMode::Sequential,
            memory_type: 0,
        }
    }
}

pub struct BufferCreateInfo {
    /// The expected usage of the buffer. This allows the buffer allocator to
    /// optimize the memory allocation, and some functionality requires specific
    /// usage flags to be set (e.g. vulkan transfer operations).
    pub usage: BufferUsageInfo,

    /// The kind of buffer to create.
    pub kind: BufferKind,

    /// The size of the buffer, in bytes. If no alignment is required, this
    /// should be set to 0.
    pub alignment: usize,

    /// The size of the buffer, in bytes.
    pub size: usize,
}

impl Default for BufferCreateInfo {
    fn default() -> Self {
        Self {
            usage: BufferUsageInfo::default(),
            kind: BufferKind::None,
            alignment: 0,
            size: 0,
        }
    }
}
