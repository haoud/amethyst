pub mod camera;

pub mod render {
    pub use amethyst_render::*;
}

pub mod vulkan {
    pub use amethyst_vulkan::*;
}

pub mod prelude {
    pub use crate::camera::{Camera3D, FlyCam, PlayerPlugin};
}
