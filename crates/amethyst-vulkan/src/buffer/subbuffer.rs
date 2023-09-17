use crate::{
    command::{Command, CommandCreateInfo, CopyBufferInfo, Idle},
    device::RenderDevice,
    prelude::QueueSubmitInfo,
};
use std::{marker::PhantomData, sync::Arc};
use vulkanalia::prelude::v1_2::*;

use super::{Buffer, BufferCreateInfo, BufferKind, BufferMemoryLocation, BufferUsageInfo};

/// A sub buffer.
pub struct SubBuffer<T> {
    /// The type of the objects in the buffer.
    marker: PhantomData<T>,

    /// The underlying buffer.
    buffer: Arc<Buffer>,

    /// The starting offset of the sub buffer in the buffer, in bytes.
    offset: usize,

    /// The number of objects of type `T` in the buffer.
    count: usize,
}

impl<T: Sized> SubBuffer<T> {
    /// Create a new buffer. The buffer will be allocated using the buffer allocator
    /// of the logical device.
    ///
    /// The count field of the sub buffer create info is overwritten by the length of
    /// the data passed as argument.
    #[must_use]
    pub fn new(device: Arc<RenderDevice>, data: &[T], mut info: SubBufferCreateInfo<T>) -> Self {
        info.count = data.len();
        let buffer = Self::empty(device, info);

        buffer.update(SubBufferUpdateInfo { data });
        buffer
    }

    /// Create a new empty buffer. The buffer will be allocated using the buffer allocator
    /// of the logical device.
    #[must_use]
    pub fn empty(device: Arc<RenderDevice>, info: SubBufferCreateInfo<T>) -> Self {
        let offset = 0;
        let count = info.count;

        let buffer = Arc::new(Buffer::empty(device, BufferCreateInfo::from(info)));
        Self {
            marker: PhantomData,
            buffer,
            offset,
            count,
        }
    }

    /// Update the buffer data. If the buffer is host visible, the data will be
    /// simply copied to the mapped memory. Otherwise, a staging buffer will be
    /// created as well as a command buffer to copy the data to the device local
    /// memory. This function will block until the data is copied to the device
    /// local memory.
    ///
    /// # Panics
    /// Panics if the size of the data is not equal to the size of the buffer.
    pub fn update(&self, info: SubBufferUpdateInfo<'_, T>) {
        assert!(info.data.len() == self.count);

        let allocation_info = unsafe {
            self.buffer
                .device
                .buffer_allocator()
                .inner()
                .get_allocation_info(&self.buffer.allocation)
                .expect("Failed to get allocation info")
        };

        unsafe {
            match self.buffer.info.usage.location {
                // The buffer is not host visible, so we need to create a staging
                // buffer before copying the data to the device local memory.
                BufferMemoryLocation::PreferDeviceLocal => {
                    // Create a staging buffer.
                    let staging = SubBuffer::new(
                        Arc::clone(&self.buffer.device),
                        info.data,
                        SubBufferCreateInfo {
                            usage: BufferUsageInfo::STAGING,
                            kind: self.buffer.info.kind,
                            count: self.count,
                            ..Default::default()
                        },
                    );

                    // Create and record the command buffer to copy the data from the staging
                    // buffer to the device local memory.
                    let command = Command::<Idle>::new(
                        Arc::clone(&self.buffer.device),
                        CommandCreateInfo {
                            ..Default::default()
                        },
                    );

                    let command = command
                        .start_recording()
                        .copy_buffer(CopyBufferInfo {
                            count: self.count,
                            src: &staging,
                            dst: &self,
                        })
                        .stop_recording();

                    // Submit the command buffer to the graphic queue.
                    self.buffer.device.graphic_queue().submit(
                        &self.buffer.device,
                        QueueSubmitInfo {
                            signal_semaphore: &[],
                            wait_semaphore: &[],
                            commands: &[&command],
                        },
                    );

                    // Wait for the command buffer to finish executing.
                    self.buffer
                        .device
                        .graphic_queue()
                        .wait_idle(&self.buffer.device);
                }

                // The buffer is host visible & coherent, we can simply copy the
                // data to the mapped memory.
                BufferMemoryLocation::PreferHostVisible => {
                    core::ptr::copy_nonoverlapping(
                        info.data.as_ptr(),
                        allocation_info.mapped_data as *mut T,
                        info.data.len(),
                    );
                }
            }
        }
    }
}

impl<T> SubBuffer<T> {
    /// Returns the inner Vulkan buffer.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::Buffer {
        self.buffer.inner
    }

    /// Returns the offset of the sub buffer.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Returns the underlying buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Returns the number of objects of type `T` in the buffer.
    #[must_use]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the size of the sub buffer.
    #[must_use]
    pub fn size(&self) -> usize {
        self.count * std::mem::size_of::<T>()
    }
}

impl From<Buffer> for SubBuffer<u8> {
    fn from(buffer: Buffer) -> Self {
        let count = buffer.info.size;
        let offset = 0;

        Self {
            buffer: Arc::new(buffer),
            marker: PhantomData,
            offset,
            count,
        }
    }
}

pub struct SubBufferCreateInfo<T> {
    /// The type of the objects in the buffer.
    pub marker: PhantomData<T>,

    /// The expected usage of the buffer. This allows the buffer allocator to
    /// optimize the memory allocation, and some functionality requires specific
    /// usage flags to be set (e.g. vulkan transfer operations).
    pub usage: BufferUsageInfo,

    /// The kind of buffer to create.
    pub kind: BufferKind,

    /// The number of objects of type `T` in the buffer.
    pub count: usize,
}

impl<T> Default for SubBufferCreateInfo<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
            usage: BufferUsageInfo::STATIC_RENDERING,
            kind: BufferKind::None,
            count: 0,
        }
    }
}

impl<T> From<SubBufferCreateInfo<T>> for BufferCreateInfo {
    fn from(value: SubBufferCreateInfo<T>) -> Self {
        Self {
            size: value.count * std::mem::size_of::<T>(),
            alignment: std::mem::align_of::<T>(),
            usage: value.usage,
            kind: value.kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubBufferUpdateInfo<'a, T> {
    /// The data to copy to the buffer.
    pub data: &'a [T],
}

impl<T> Default for SubBufferUpdateInfo<'_, T> {
    fn default() -> Self {
        Self { data: &[] }
    }
}
