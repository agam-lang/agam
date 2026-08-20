//! Scene Graph Management and Fixed-Timestep Physics Systems.

use crate::components::{RigidBody, Transform};
use crate::ecs::{Entity, World};

/// Frame time and physics simulation metrics.
#[derive(Clone, Copy, Debug, Default)]
pub struct GameTime {
    pub delta_seconds: f32,
    pub total_seconds: f64,
    pub frame_count: u64,
}

/// A playable scene managing entities and systems.
pub struct Scene {
    pub name: String,
    pub world: World,
    pub time: GameTime,
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            world: World::new(),
            time: GameTime::default(),
        }
    }

    /// Run one step of Euler physics integration for an entity with Transform and RigidBody.
    pub fn step_physics_entity(&mut self, entity: Entity, dt: f32) {
        if let Some(rb) = self.world.get_mut::<RigidBody>(entity) {
            rb.integrate(dt);
            let vel = rb.velocity;
            if let Some(t) = self.world.get_mut::<Transform>(entity) {
                t.translation = t.translation + (vel * dt);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    #[test]
    fn test_physics_step_moves_transform() {
        let mut scene = Scene::new("TestWorld");
        let e = scene.world.spawn();

        let rb = RigidBody {
            velocity: Vec3::new(10.0, 0.0, 0.0),
            ..Default::default()
        };

        scene.world.insert(e, Transform::default());
        scene.world.insert(e, rb);

        scene.step_physics_entity(e, 0.5);

        let t = scene.world.get::<Transform>(e).unwrap();
        assert_eq!(t.translation.x, 5.0);
    }
}
