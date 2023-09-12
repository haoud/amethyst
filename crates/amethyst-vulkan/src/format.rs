use vulkanalia::prelude::v1_2::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Undefined,

    R32G32B32A32SFLOAT,
    R32G32B32SFLOAT,
    R32G32SFLOAT,
    R32SFLOAT,

    R8G8B8A8SRGB,
    B8G8R8A8SRGB,

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
