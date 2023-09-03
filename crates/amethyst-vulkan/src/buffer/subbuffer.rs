use crate::{
    command::{Command, CommandCreateInfo, CopyBufferInfo, Idle},
    device::RenderDevice,
    prelude::QueueSubmitInfo,
};
use std::{marker::PhantomData, mem::ManuallyDrop, sync::Arc};
use vk_mem_vulkanalia::{Alloc, AllocationCreateFlags, AllocationCreateInfo};
use vulkanalia::prelude::v1_2::*;

use super::{Buffer, BufferAccessMode, BufferKind, BufferMemoryLocation, BufferTransfert};

/// A sub buffer.
pub struct SubBuffer<T: ?Sized> {
    marker: PhantomData<T>,
    info: SubBufferCreateInfo,
    buffer: Buffer,
    offset: usize,
    size: usize,
}

impl<T: Sized> SubBuffer<T> {
    /// Create a new buffer. The buffer will be allocated using the buffer allocator
    /// of the logical device.
    #[must_use]
    pub fn new(
        device: Arc<RenderDevice>,
        data: &[T],
        kind: BufferKind,
        info: SubBufferCreateInfo,
    ) -> Self {
        let transfert = vk::BufferUsageFlags::from(info.transfer);
        let flags = vk::BufferUsageFlags::from(kind);
        let size = std::mem::size_of::<T>() * data.len();

        let mut create_info = AllocationCreateInfo::from(info.location);
        create_info.flags |= AllocationCreateFlags::from(info.access);

        let (buffer, allocation) = unsafe {
            device
                .buffer_allocator()
                .inner()
                .create_buffer(
                    &vk::BufferCreateInfo::builder()
                        .usage(transfert | flags)
                        .size(size as u64)
                        .build(),
                    &create_info,
                )
                .expect("Failed to create buffer")
        };

        let buffer = Buffer {
            allocation: ManuallyDrop::new(allocation),
            device: Arc::clone(&device),
            inner: buffer,
            kind,
        };

        let view = Self {
            marker: PhantomData,
            offset: 0,
            buffer,
            info,
            size,
        };

        view.update(SubBufferUpdateInfo { data });

        view
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
        assert!(info.data.len() * std::mem::size_of::<T>() == self.size);

        let allocation_info = unsafe {
            self.buffer
                .device
                .buffer_allocator()
                .inner()
                .get_allocation_info(&self.buffer.allocation)
                .expect("Failed to get allocation info")
        };

        unsafe {
            match self.info.location {
                // The buffer is not host visible, so we need to create a staging
                // buffer before copying the data to the device local memory.
                BufferMemoryLocation::PreferDeviceLocal => {
                    // Create a staging buffer.
                    let staging = SubBuffer::new(
                        Arc::clone(&self.buffer.device),
                        info.data,
                        self.buffer.kind,
                        SubBufferCreateInfo::STAGING,
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
                            size: self.size as u64,
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

impl<T: ?Sized> SubBuffer<T> {}

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

    /// Returns the buffer create info.
    pub fn info(&self) -> &SubBufferCreateInfo {
        &self.info
    }

    /// Returns the size of the sub buffer.
    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }
}

/// A buffer create info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubBufferCreateInfo {
    pub location: BufferMemoryLocation,
    pub transfer: BufferTransfert,
    pub access: BufferAccessMode,
}

impl SubBufferCreateInfo {
    /// A pre-defined buffer create info for static mesh rendering. This buffer
    /// will be stored in device local memory and data will be copied to it once
    /// using a staging buffer.
    pub const STATIC_RENDERING: SubBufferCreateInfo = SubBufferCreateInfo {
        location: BufferMemoryLocation::PreferDeviceLocal,
        transfer: BufferTransfert::Destination,
        access: BufferAccessMode::None,
    };

    /// A pre-defined buffer create info for creating a staging buffer. This
    /// buffer will be stored in host visible memory and can be used to copy
    /// data to device local memory.
    pub const STAGING: SubBufferCreateInfo = SubBufferCreateInfo {
        location: BufferMemoryLocation::PreferHostVisible,
        transfer: BufferTransfert::Source,
        access: BufferAccessMode::Sequential,
    };

    /// A pre-defined buffer create info for creating a uniform buffer. This
    /// buffer will be stored in host visible memory and can be used to update
    /// the buffer data frequently.
    pub const UNIFORM: SubBufferCreateInfo = SubBufferCreateInfo {
        location: BufferMemoryLocation::PreferDeviceLocal,
        transfer: BufferTransfert::Destination,
        access: BufferAccessMode::Sequential,
    };
}

impl Default for SubBufferCreateInfo {
    fn default() -> Self {
        Self {
            transfer: BufferTransfert::Destination,
            access: BufferAccessMode::Sequential,
            location: BufferMemoryLocation::PreferDeviceLocal,
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
