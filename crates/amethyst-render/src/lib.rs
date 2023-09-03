use amethyst_vulkan::prelude::*;

pub mod camera;
pub mod frame;
pub mod prelude;

/// A simple vertex that contains a 2D position.
#[repr(C)]
pub struct Vertex2D {
    pub position: [f32; 2],
}

/// A simple vertex that contains a 2D position and a RGB color.
#[repr(C)]
pub struct Vertex2DColor {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

unsafe impl VertexBindingDescription for Vertex2DColor {
    fn binding_description() -> BindingDescription {
        BindingDescription {
            stride: std::mem::size_of::<Vertex2DColor>() as u32,
            binding: 0,
        }
    }
}

unsafe impl VertexAttributeDescription for Vertex2DColor {
    fn attribute_descriptions() -> Vec<AttributeDescription> {
        vec![
            // Describe the position attribute.
            AttributeDescription {
                format: Format::R32G32Sfloat,
                location: 0,
                binding: 0,
                offset: 0,
            },
            // Describe the color attribute.
            AttributeDescription {
                format: Format::R32G32B32Sfloat,
                location: 1,
                binding: 0,
                offset: 8,
            },
        ]
    }
}

/// A simple vertex that contains a 3D position and a RGB color.
#[repr(C)]
pub struct Vertex3DColor {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

unsafe impl VertexBindingDescription for Vertex3DColor {
    fn binding_description() -> BindingDescription {
        BindingDescription {
            stride: std::mem::size_of::<Vertex3DColor>() as u32,
            binding: 0,
        }
    }
}

unsafe impl VertexAttributeDescription for Vertex3DColor {
    fn attribute_descriptions() -> Vec<AttributeDescription> {
        vec![
            // Describe the position attribute.
            AttributeDescription {
                format: Format::R32G32B32Sfloat,
                location: 0,
                binding: 0,
                offset: 0,
            },
            // Describe the color attribute.
            AttributeDescription {
                format: Format::R32G32B32Sfloat,
                location: 1,
                binding: 0,
                offset: 12,
            },
        ]
    }
}
