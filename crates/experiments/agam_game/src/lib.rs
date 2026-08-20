//! # agam_game
//!
//! Modern 2D/3D Game Engine with Entity-Component-System (ECS) Architecture.
//!
//! Provides:
//! - **Linear Algebra & Math (`math`)**: High-performance `Vec2`, `Vec3`, `Vec4`, `Mat4`, `Quat`.
//! - **ECS Core (`ecs`)**: Generational `Entity`, type-safe `Component`, and `World`.
//! - **Engine Components (`components`)**: `Transform`, `Camera`, `RigidBody`.
//! - **Scene & Simulation (`scene`)**: `Scene` graph and physics simulation.

pub mod components;
pub mod ecs;
pub mod math;
pub mod scene;

pub use components::{Camera, RigidBody, Transform};
pub use ecs::{Component, Entity, World};
pub use math::{Mat4, Quat, Vec2, Vec3};
pub use scene::{GameTime, Scene};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_engine_smoke() {
        let mut scene = Scene::new("SpaceGame");
        let player = scene.world.spawn();

        scene.world.insert(
            player,
            Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
        );
        let mut rb = RigidBody::default();
        rb.apply_force(Vec3::new(0.0, 10.0, 0.0));
        scene.world.insert(player, rb);

        scene.step_physics_entity(player, 1.0);

        let t = scene.world.get::<Transform>(player).unwrap();
        assert_eq!(t.translation.y, 11.0);
    }
}
