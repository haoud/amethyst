use amethyst_ecs::world::World;

pub struct Engine {
    world: World,
}

impl Engine {
    #[must_use]
    pub fn new(info: EngineInfo) -> Self {
        Self { world: info.world }
    }

    #[must_use]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            world: World::new(),
        }
    }
}

pub struct EngineInfo {
    pub world: World,
}

impl Default for EngineInfo {
    fn default() -> Self {
        Self {
            world: World::new(),
        }
    }
}
