use amethyst_vulkan::pipeline::{VertexAttributeDescription, VertexBindingDescription};
use vulkanalia::prelude::v1_3::*;

/// A simple vertex that contains a 2D position and a RGB color.
#[derive(Default, Debug, Clone)]
#[repr(C)]
pub struct Vertex2DColor {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

unsafe impl VertexBindingDescription for Vertex2DColor {
    fn binding_description() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
            binding: 0,
        }]
    }
}

unsafe impl VertexAttributeDescription for Vertex2DColor {
    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // Describe the position attribute.
            vk::VertexInputAttributeDescription {
                offset: core::mem::offset_of!(Self, position) as u32,
                format: vk::Format::R32G32_SFLOAT,
                location: 0,
                binding: 0,
            },
            // Describe the color attribute.
            vk::VertexInputAttributeDescription {
                offset: core::mem::offset_of!(Self, color) as u32,
                format: vk::Format::R32G32B32_SFLOAT,
                location: 1,
                binding: 0,
            },
        ]
    }
}
