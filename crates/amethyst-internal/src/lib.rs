pub mod camera;

pub mod vulkan {
    pub use amethyst_vulkan as vulkan;
}

pub mod prelude {
    pub use crate::camera::{Camera3D, FlyCam, PlayerPlugin};
}
