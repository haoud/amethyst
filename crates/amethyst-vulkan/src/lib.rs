pub mod buffer;
pub mod command;
pub mod context;
pub mod device;
pub mod pipeline;
pub mod semaphore;
pub mod shader;
pub mod swapchain;

pub mod vk {
    pub use vulkanalia::prelude::v1_3::vk::*;
}

/// The maximum number of frames that can be in flight at once.
pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
