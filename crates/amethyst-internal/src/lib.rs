pub mod prelude {
    pub use crate::ecs as bevy_ecs;
    pub use crate::{core::prelude::*, ecs::prelude::*, math::prelude::*};

    #[cfg(feature = "amethyst-render")]
    pub use crate::render::prelude::*;

    #[cfg(feature = "amethyst-window")]
    pub use crate::window::prelude::*;

    #[cfg(feature = "amethyst-vulkan")]
    pub use crate::vulkan::prelude::*;
}

pub mod core {
    pub use amethyst_core::*;
}

pub mod ecs {
    pub use amethyst_ecs::*;
}

pub mod math {
    pub use amethyst_math::*;
}

#[cfg(feature = "amethyst-render")]
pub mod render {
    pub use amethyst_render::*;
}

#[cfg(feature = "amethyst-vulkan")]
pub mod vulkan {
    pub use amethyst_vulkan::*;
}

#[cfg(feature = "amethyst-window")]
pub mod window {
    pub use amethyst_window::*;
}
