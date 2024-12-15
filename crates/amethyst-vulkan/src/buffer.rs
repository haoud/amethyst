use crate::{context::VulkanContext, device::VulkanDevice};
use std::sync::Arc;
use vma::Alloc;
use vulkanalia::prelude::v1_3::*;

/// A buffer allocator that uses the Vulkan Memory Allocator library.
/// This is a simple and thin wrapper around the VMA allocator.
#[derive(Debug)]
pub struct BufferAllocator {
    inner: vma::Allocator,
}

impl BufferAllocator {
    /// Create a new buffer allocator with default settings.
    #[must_use]
    pub fn new(context: &VulkanContext, device: &VulkanDevice) -> Self {
        // Create the buffer allocator. It use the Vulkan Memory Allocator library
        // with rust bindings.
        let inner = unsafe {
            vma::Allocator::new(&vma::AllocatorOptions::new(
                &context.instance(),
                device.logical(),
                device.physical(),
            ))
            .expect("Failed to create buffer allocator")
        };

        Self { inner }
    }

    /// Get a reference to the inner allocator.
    #[must_use]
    pub const fn inner(&self) -> &vma::Allocator {
        &self.inner
    }
}

/// A buffer object that can be used to store data on the GPU.
#[derive(Debug)]
pub struct Buffer {
    /// The buffer allocator that allocated this buffer.
    allocator: Arc<BufferAllocator>,

    /// The allocation information of this buffer, such as the start offset,
    /// size, and memory type.
    allocation: vma::Allocation,

    /// The buffer in which this buffer belongs to. This object is not owned by
    /// this buffer, and other buffer can share the same buffer, but with a
    /// different allocation (start offset and size).
    buffer: vk::Buffer,
}

impl Buffer {
    /// Create a new buffer with the given device, allocator, and buffer creation
    /// information.
    #[must_use]
    pub fn new<T>(allocator: Arc<BufferAllocator>, create_info: BufferCreateInfo<T>) -> Self {
        // Create the allocation information for the buffer from our splitted
        // buffer information that allow a better API design.
        let mut allocation_info = vma::AllocationOptions::from(create_info.usage.location);
        allocation_info.flags |= vma::AllocationCreateFlags::from(create_info.usage.access);
        allocation_info.memory_type_bits = create_info.usage.memory_type;

        let usage = vk::BufferUsageFlags::from(create_info.usage.transfer)
            | vk::BufferUsageFlags::from(create_info.usage.usage);
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(create_info.data.size() as vk::DeviceSize)
            .usage(usage);

        // Create the buffer with the allocator.
        let (buffer, allocation) = unsafe {
            allocator
                .inner()
                .create_buffer_with_alignment(
                    buffer_info,
                    &allocation_info,
                    create_info.alignment as vk::DeviceSize,
                )
                .expect("Failed to create buffer")
        };

        // Copy the data to the buffer if it is provided.
        if let BufferDataInfo::Slice(data) = create_info.data {
            match create_info.usage.location {
                BufferMemoryLocation::PreferDeviceLocal => {
                    // Create a staging buffer
                    // Copy the data to the staging buffer
                    // Copy the data from the staging buffer to the device local buffer
                    // using a command buffer.
                    todo!()
                }
                BufferMemoryLocation::PreferHostVisible => {
                    let allocation_info = allocator.inner().get_allocation_info(allocation);
                    let ptr = allocation_info.pMappedData as *mut T;
                    unsafe {
                        assert!(!ptr.is_null());
                        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
                    }
                }
            }
        }

        Self {
            allocator,
            allocation,
            buffer,
        }
    }

    /// Get the start offset of this buffer inside the `vk::Buffer` object.
    #[must_use]
    pub fn start_offset(&self) -> vk::DeviceSize {
        self.allocator
            .inner
            .get_allocation_info(self.allocation)
            .offset
    }

    /// Get the size of this buffer.
    #[must_use]
    pub fn size(&self) -> vk::DeviceSize {
        self.allocator
            .inner
            .get_allocation_info(self.allocation)
            .size
    }

    /// Return the buffer allocator that allocated this buffer.
    #[must_use]
    pub fn allocator(&self) -> &Arc<BufferAllocator> {
        &self.allocator
    }

    /// Return the inner buffer object that this buffer belongs to. Please note
    /// that the buffer object is not owned by this buffer, and other buffer
    /// can share the same buffer, but with a different allocation (start offset
    /// and size).
    #[must_use]
    pub const fn inner(&self) -> vk::Buffer {
        self.buffer
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.allocator
                .inner
                .destroy_buffer(self.buffer, self.allocation);
        }
    }
}

/// The usage of the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferUsage {
    /// The buffer will not be used for storing any kind of data. Instead, it will be used
    /// as a source or destination for a transfer operation.
    None,

    /// The buffer will be used for storing uniform data.
    Uniforms,

    /// The buffer will be used for storing vertex data.
    Vertices,

    /// The buffer will be used for storing index data.
    Indices,

    /// The buffer will be used for storing data.
    Storage,

    /// The buffer can be used for any purpose. This is useful for buffers that
    /// are used for multiple purposes, or when the buffer usage is not known
    /// at the time of creation, but can restrict the buffer allocator to use
    /// the most optimal memory location and may cause a performance penalty.
    Unbounded,
}

impl From<BufferUsage> for vk::BufferUsageFlags {
    fn from(kind: BufferUsage) -> Self {
        match kind {
            BufferUsage::Uniforms => vk::BufferUsageFlags::UNIFORM_BUFFER,
            BufferUsage::Vertices => vk::BufferUsageFlags::VERTEX_BUFFER,
            BufferUsage::Storage => vk::BufferUsageFlags::STORAGE_BUFFER,
            BufferUsage::Indices => vk::BufferUsageFlags::INDEX_BUFFER,
            BufferUsage::Unbounded => vk::BufferUsageFlags::all(),
            BufferUsage::None => vk::BufferUsageFlags::empty(),
        }
    }
}

/// Memory location where the buffer should be allocated.
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

impl From<BufferMemoryLocation> for vma::AllocationOptions {
    fn from(location: BufferMemoryLocation) -> Self {
        match location {
            BufferMemoryLocation::PreferHostVisible => Self {
                required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT,
                preferred_flags: vk::MemoryPropertyFlags::HOST_CACHED,
                flags: vma::AllocationCreateFlags::MAPPED,
                usage: vma::MemoryUsage::AutoPreferHost,
                ..Default::default()
            },
            BufferMemoryLocation::PreferDeviceLocal => Self {
                usage: vma::MemoryUsage::AutoPreferDevice,
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
pub enum BufferAccess {
    /// The buffer will not directly be accessed by the CPU.
    None,

    /// The buffer will be accessed by the CPU in a sequential manner, for example
    /// by used `std::ptr::copy_nonoverlapping` to copy data to the buffer. Watch out
    /// for implicit reads introduced by writing to a buffer, for example when
    /// incrementing or decrementing a value in a buffer.
    Sequential,

    /// The buffer will be accessed by the CPU in a random manner. The memory
    /// allocated can be read and written to in any order.
    Random,
}

impl From<BufferAccess> for vma::AllocationCreateFlags {
    fn from(mode: BufferAccess) -> Self {
        match mode {
            BufferAccess::Sequential => vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            BufferAccess::Random => vma::AllocationCreateFlags::HOST_ACCESS_RANDOM,
            BufferAccess::None => vma::AllocationCreateFlags::empty(),
        }
    }
}

/// Information about the buffer usage.
#[derive(Debug)]
pub struct BufferUsageInfo {
    /// The location where the buffer should be allocated, either in device local
    /// memory or host visible memory. See the [`BufferMemoryLocation`] enum for
    /// more information.
    pub location: BufferMemoryLocation,

    /// The buffer transfer usage. This allows the buffer allocator to optimize
    /// the memory allocation location depending on the transfer mode. See the
    /// [`BufferTransfert`] enum for more information.
    pub transfer: BufferTransfert,

    /// The buffer access mode. This allows the buffer allocator to optimize the
    /// memory allocation location depending on the access mode. See the
    /// [`BufferAccessMode`] enum for more information.
    pub access: BufferAccess,

    /// The usage of the buffer. This allows the buffer allocator to optimize the
    /// memory allocation location depending on the buffer usage. See the
    /// [`BufferUsage`] enum for more information.
    pub usage: BufferUsage,

    /// The memory type bits of the buffer. This is a bitmask containing one bit set for
    /// every memory type acceptable for this allocation. If the value is 0, it is ignored
    /// and all memory types are considered acceptable if all other conditions are met.
    pub memory_type: u32,
}

impl Default for BufferUsageInfo {
    fn default() -> Self {
        Self {
            location: BufferMemoryLocation::PreferDeviceLocal,
            transfer: BufferTransfert::All,
            access: BufferAccess::Random,
            usage: BufferUsage::Unbounded,
            memory_type: 0,
        }
    }
}

/// Information required to create a buffer.
#[derive(Debug)]
pub struct BufferCreateInfo<'a, T> {
    /// Usage information of the buffer.
    pub usage: BufferUsageInfo,

    /// The minimum alignment of the buffer.
    pub alignment: usize,

    /// The data of the buffer.
    pub data: BufferDataInfo<'a, T>,
}

impl<T> Default for BufferCreateInfo<'_, T> {
    fn default() -> Self {
        Self {
            usage: BufferUsageInfo::default(),
            alignment: core::mem::align_of::<T>(),
            data: BufferDataInfo::Uninitialized(0),
        }
    }
}

/// Information about the buffer data.
#[derive(Debug)]
pub enum BufferDataInfo<'a, T> {
    /// Create a buffer with uninitialized data with the given size.
    Uninitialized(usize),

    /// Create a buffer with the given data slice.
    Slice(&'a [T]),
}

impl<T> BufferDataInfo<'_, T> {
    /// Get the number of `T` elements in the buffer data
    #[must_use]
    pub const fn count(&self) -> usize {
        match self {
            BufferDataInfo::Uninitialized(size) => *size / std::mem::size_of::<T>(),
            BufferDataInfo::Slice(data) => data.len(),
        }
    }

    /// Get the size of the buffer data, in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        match self {
            BufferDataInfo::Uninitialized(size) => *size,
            BufferDataInfo::Slice(data) => data.len() * std::mem::size_of::<T>(),
        }
    }
}
