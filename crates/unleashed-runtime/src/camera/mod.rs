//! Debug/free-fly camera state.
//!
//! This module intentionally depends only on `std` so it can be wired to the
//! platform and renderer without adding a runtime-crate math dependency.

pub mod math;

use std::f32::consts::FRAC_PI_2;

const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.001;

/// Per-frame input consumed by [`FreeFlyCamera::update`].
///
/// Movement is camera-local:
/// - `movement[0]`: right (+) / left (-)
/// - `movement[1]`: up (+) / down (-)
/// - `movement[2]`: forward (+) / backward (-)
///
/// `look_delta` is `[yaw, pitch]` in input units. `look_sensitivity` converts
/// those units to radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeFlyInput {
    pub movement: [f32; 3],
    pub look_delta: [f32; 2],
    pub boost: bool,
}

impl Default for FreeFlyInput {
    fn default() -> Self {
        Self {
            movement: [0.0; 3],
            look_delta: [0.0; 2],
            boost: false,
        }
    }
}

/// Orthonormal world-space basis derived from a free-fly camera orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraBasis {
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
}

/// Minimal camera state for detached stage inspection.
///
/// Coordinates use +Y as world up. At zero yaw/pitch the camera looks toward
/// -Z and +X is camera-right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeFlyCamera {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub move_speed: f32,
    pub boost_multiplier: f32,
    pub look_sensitivity: f32,
}

impl Default for FreeFlyCamera {
    fn default() -> Self {
        Self {
            position: [0.0, 2.0, 5.0],
            yaw: 0.0,
            pitch: 0.0,
            move_speed: 12.0,
            boost_multiplier: 4.0,
            look_sensitivity: 0.0025,
        }
    }
}

impl FreeFlyCamera {
    pub const fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: 0.0,
            move_speed: 12.0,
            boost_multiplier: 4.0,
            look_sensitivity: 0.0025,
        }
    }

    /// Repositions and reorients the camera while preserving tuning values.
    pub fn set_pose(&mut self, position: [f32; 3], yaw: f32, pitch: f32) {
        self.position = position;
        self.yaw = yaw;
        self.pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    pub fn basis(&self) -> CameraBasis {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();

        let forward = [sin_yaw * cos_pitch, sin_pitch, -cos_yaw * cos_pitch];
        let right = [cos_yaw, 0.0, sin_yaw];
        let up = cross(right, forward);

        CameraBasis { forward, right, up }
    }

    /// Advances camera state without allocations.
    ///
    /// Invalid/non-positive frame deltas still apply look input but skip
    /// translation, preventing a bad timing sample from teleporting the camera.
    pub fn update(&mut self, delta_seconds: f32, input: FreeFlyInput) {
        self.yaw += input.look_delta[0] * self.look_sensitivity;
        self.pitch = (self.pitch + input.look_delta[1] * self.look_sensitivity)
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);

        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }

        let movement = normalized_or_zero(input.movement);
        if movement == [0.0; 3] {
            return;
        }

        let basis = self.basis();
        let direction = add3(
            add3(
                scale3(basis.right, movement[0]),
                scale3(basis.up, movement[1]),
            ),
            scale3(basis.forward, movement[2]),
        );

        let speed = self.move_speed
            * if input.boost {
                self.boost_multiplier
            } else {
                1.0
            };

        self.position = add3(self.position, scale3(direction, speed * delta_seconds));
    }
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(v: [f32; 3], scale: f32) -> [f32; 3] {
    [v[0] * scale, v[1] * scale, v[2] * scale]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalized_or_zero(v: [f32; 3]) -> [f32; 3] {
    let length_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if !length_sq.is_finite() || length_sq <= f32::EPSILON {
        return [0.0; 3];
    }

    scale3(v, length_sq.sqrt().recip())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_basis_faces_negative_z() {
        let basis = FreeFlyCamera::default().basis();
        assert_eq!(basis.forward, [0.0, 0.0, -1.0]);
        assert_eq!(basis.right, [1.0, 0.0, 0.0]);
        assert_eq!(basis.up, [-0.0, 1.0, 0.0]);
    }

    #[test]
    fn diagonal_movement_is_normalized() {
        let mut camera = FreeFlyCamera::new([0.0; 3]);
        camera.move_speed = 1.0;
        camera.update(
            1.0,
            FreeFlyInput {
                movement: [1.0, 0.0, 1.0],
                ..FreeFlyInput::default()
            },
        );

        let distance = (camera.position[0] * camera.position[0]
            + camera.position[1] * camera.position[1]
            + camera.position[2] * camera.position[2])
            .sqrt();
        assert!((distance - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn pitch_is_clamped() {
        let mut camera = FreeFlyCamera::default();
        camera.look_sensitivity = 1.0;
        camera.update(
            0.0,
            FreeFlyInput {
                look_delta: [0.0, 100.0],
                ..FreeFlyInput::default()
            },
        );
        assert!(camera.pitch < FRAC_PI_2);
    }
}
