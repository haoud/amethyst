use vulkanalia::prelude::v1_2::*;

/// Represents different types of image/data formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Undefined format.
    Undefined,

    /// 128-bit format containing 32-bit Red, Green, Blue and Alpha components.
    R32G32B32A32SFLOAT,

    /// 96-bit format containing 32-bit Red, Green and Blue components.
    R32G32B32SFLOAT,

    /// 64-bit format containing 32-bit Red and Green components.
    R32G32SFLOAT,

    /// 32-bit format containing a single 32-bit Red component.
    R32SFLOAT,

    /// 32-bit format containing 8-bit Red, Green, Blue and Alpha components.
    R8G8B8A8SRGB,

    /// 32-bit format containing 8-bit Blue, Green, Red and Alpha components.
    B8G8R8A8SRGB,

    /// 32-bit format containing a single 32-bit depth component.
    D32SFLOAT,
}

impl From<Format> for vk::Format {
    fn from(value: Format) -> Self {
        match value {
            Format::R32G32B32A32SFLOAT => vk::Format::R32G32B32A32_SFLOAT,
            Format::R32G32B32SFLOAT => vk::Format::R32G32B32_SFLOAT,
            Format::R32G32SFLOAT => vk::Format::R32G32_SFLOAT,
            Format::R32SFLOAT => vk::Format::R32_SFLOAT,
            Format::R8G8B8A8SRGB => vk::Format::R8G8B8A8_SRGB,
            Format::B8G8R8A8SRGB => vk::Format::B8G8R8A8_SRGB,
            Format::D32SFLOAT => vk::Format::D32_SFLOAT,
            _ => vk::Format::UNDEFINED,
        }
    }
}

impl From<vk::Format> for Format {
    fn from(value: vk::Format) -> Self {
        match value {
            vk::Format::R32G32B32A32_SFLOAT => Self::R32G32B32A32SFLOAT,
            vk::Format::R32G32B32_SFLOAT => Self::R32G32B32SFLOAT,
            vk::Format::R32G32_SFLOAT => Self::R32G32SFLOAT,
            vk::Format::R32_SFLOAT => Self::R32SFLOAT,
            vk::Format::R8G8B8A8_SRGB => Self::R8G8B8A8SRGB,
            vk::Format::B8G8R8A8_SRGB => Self::B8G8R8A8SRGB,
            vk::Format::D32_SFLOAT => Self::D32SFLOAT,
            _ => Self::Undefined,
        }
    }
}
