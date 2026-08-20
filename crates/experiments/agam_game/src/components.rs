//! Standard Game Engine Component Library (Transform, Camera, RigidBody, MeshRenderer).

use crate::math::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Spatial hierarchy transform component.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Default::default()
        }
    }

    /// Compute the 4x4 affine model transform matrix.
    pub fn model_matrix(&self) -> Mat4 {
        let t = Mat4::from_translation(self.translation);
        let s = Mat4::from_scale(self.scale);
        t.mul_mat4(s)
    }
}

/// Perspective or Orthographic Camera Component.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub fov_radians: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov_radians: 60.0f32.to_radians(),
            aspect_ratio: 16.0 / 9.0,
            z_near: 0.1,
            z_far: 1000.0,
        }
    }
}

impl Camera {
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov_radians, self.aspect_ratio, self.z_near, self.z_far)
    }
}

/// Physics RigidBody Component.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RigidBody {
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub mass: f32,
    pub restitution: f32,
    pub is_kinematic: bool,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            acceleration: Vec3::ZERO,
            mass: 1.0,
            restitution: 0.2,
            is_kinematic: false,
        }
    }
}

impl RigidBody {
    pub fn apply_force(&mut self, force: Vec3) {
        if !self.is_kinematic && self.mass > 0.0 {
            self.acceleration = self.acceleration + (force * (1.0 / self.mass));
        }
    }

    pub fn integrate(&mut self, delta_seconds: f32) {
        if !self.is_kinematic {
            self.velocity = self.velocity + (self.acceleration * delta_seconds);
            self.acceleration = Vec3::ZERO;
        }
    }
}
