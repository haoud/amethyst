use amethyst::{prelude::*, render::AmethystRender};
use bevy::prelude::*;

/// This example illustrates how to create a simple amethyst application with a
/// window and a player. This example is exactly the same starting point as an
/// bevy application (since Amethyst is built on top of bevy ECS) but with the
/// addition of the `PlayerPlugin` which adds a player to the scene.
pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (800.0, 600.0).into(),
                title: "Amesthyst".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PlayerPlugin)
        .add_plugins(AmethystRender)
        .run();
}
