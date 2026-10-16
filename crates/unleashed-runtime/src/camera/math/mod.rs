//! Dependency-free camera matrix generation.
//!
//! Matrices are returned as four columns (`[[f32; 4]; 4]`) to match WGSL's
//! `mat4x4<f32>` layout and `Renderer::update_terrain_camera`.
//!
//! The convention is right-handed with +Y world-up. Camera forward maps to
//! view-space -Z and perspective depth maps the near/far planes to 0/1.

use std::f32::consts::PI;
use std::fmt;

use super::FreeFlyCamera;

/// A column-major 4x4 matrix, indexed as `[column][row]`.
pub type Mat4Columns = [[f32; 4]; 4];

/// Invalid parameters for a finite right-handed 0..1 perspective projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerspectiveError {
    NonFiniteInput,
    NonPositiveAspectRatio,
    InvalidVerticalFov,
    InvalidDepthRange,
}

impl fmt::Display for PerspectiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => f.write_str("perspective input must be finite"),
            Self::NonPositiveAspectRatio => f.write_str("aspect ratio must be greater than zero"),
            Self::InvalidVerticalFov => {
                f.write_str("vertical field of view must be greater than zero and less than pi")
            }
            Self::InvalidDepthRange => {
                f.write_str("near plane must be greater than zero and far plane must exceed near")
            }
        }
    }
}

impl std::error::Error for PerspectiveError {}

/// Builds the world-to-view matrix for an existing [`FreeFlyCamera`].
///
/// The camera's existing orthonormal basis is used directly: right maps to +X,
/// up maps to +Y, and forward maps to -Z.
pub fn view_columns(camera: &FreeFlyCamera) -> Mat4Columns {
    let basis = camera.basis();
    let position = camera.position;

    [
        [basis.right[0], basis.up[0], -basis.forward[0], 0.0],
        [basis.right[1], basis.up[1], -basis.forward[1], 0.0],
        [basis.right[2], basis.up[2], -basis.forward[2], 0.0],
        [
            -dot3(basis.right, position),
            -dot3(basis.up, position),
            dot3(basis.forward, position),
            1.0,
        ],
    ]
}

/// Builds a right-handed perspective projection with a 0..1 depth range.
///
/// View-space forward is -Z. `vertical_fov_radians` is the full vertical field
/// of view, and `aspect_ratio` is width / height.
pub fn perspective_columns(
    vertical_fov_radians: f32,
    aspect_ratio: f32,
    near_plane: f32,
    far_plane: f32,
) -> Result<Mat4Columns, PerspectiveError> {
    if !vertical_fov_radians.is_finite()
        || !aspect_ratio.is_finite()
        || !near_plane.is_finite()
        || !far_plane.is_finite()
    {
        return Err(PerspectiveError::NonFiniteInput);
    }
    if aspect_ratio <= 0.0 {
        return Err(PerspectiveError::NonPositiveAspectRatio);
    }
    if vertical_fov_radians <= 0.0 || vertical_fov_radians >= PI {
        return Err(PerspectiveError::InvalidVerticalFov);
    }
    if near_plane <= 0.0 || far_plane <= near_plane {
        return Err(PerspectiveError::InvalidDepthRange);
    }

    let focal_length = (vertical_fov_radians * 0.5).tan().recip();
    let depth_scale = far_plane / (near_plane - far_plane);
    let depth_translation = near_plane * depth_scale;

    Ok([
        [focal_length / aspect_ratio, 0.0, 0.0, 0.0],
        [0.0, focal_length, 0.0, 0.0],
        [0.0, 0.0, depth_scale, -1.0],
        [0.0, 0.0, depth_translation, 0.0],
    ])
}

/// Builds the world-to-clip matrix expected by
/// `Renderer::update_terrain_camera`.
pub fn view_projection_columns(
    camera: &FreeFlyCamera,
    vertical_fov_radians: f32,
    aspect_ratio: f32,
    near_plane: f32,
    far_plane: f32,
) -> Result<Mat4Columns, PerspectiveError> {
    let projection =
        perspective_columns(vertical_fov_radians, aspect_ratio, near_plane, far_plane)?;
    Ok(mul_columns(projection, view_columns(camera)))
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn mul_columns(a: Mat4Columns, b: Mat4Columns) -> Mat4Columns {
    let mut out = [[0.0; 4]; 4];

    for column in 0..4 {
        for row in 0..4 {
            out[column][row] = a[0][row] * b[column][0]
                + a[1][row] * b[column][1]
                + a[2][row] * b[column][2]
                + a[3][row] * b[column][3];
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1.0e-5;

    fn transform(matrix: Mat4Columns, point: [f32; 4]) -> [f32; 4] {
        [
            matrix[0][0] * point[0]
                + matrix[1][0] * point[1]
                + matrix[2][0] * point[2]
                + matrix[3][0] * point[3],
            matrix[0][1] * point[0]
                + matrix[1][1] * point[1]
                + matrix[2][1] * point[2]
                + matrix[3][1] * point[3],
            matrix[0][2] * point[0]
                + matrix[1][2] * point[1]
                + matrix[2][2] * point[2]
                + matrix[3][2] * point[3],
            matrix[0][3] * point[0]
                + matrix[1][3] * point[1]
                + matrix[2][3] * point[2]
                + matrix[3][3] * point[3],
        ]
    }

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn view_maps_camera_position_to_origin_and_forward_to_negative_z() {
        let mut camera = FreeFlyCamera::new([3.0, -2.0, 7.0]);
        camera.set_pose([3.0, -2.0, 7.0], 0.73, -0.31);

        let view = view_columns(&camera);
        let at_camera = transform(
            view,
            [
                camera.position[0],
                camera.position[1],
                camera.position[2],
                1.0,
            ],
        );
        assert_near(at_camera[0], 0.0);
        assert_near(at_camera[1], 0.0);
        assert_near(at_camera[2], 0.0);
        assert_near(at_camera[3], 1.0);

        let forward = camera.basis().forward;
        let distance = 4.0;
        let ahead = transform(
            view,
            [
                camera.position[0] + forward[0] * distance,
                camera.position[1] + forward[1] * distance,
                camera.position[2] + forward[2] * distance,
                1.0,
            ],
        );
        assert_near(ahead[0], 0.0);
        assert_near(ahead[1], 0.0);
        assert_near(ahead[2], -distance);
        assert_near(ahead[3], 1.0);
    }

    #[test]
    fn perspective_and_view_projection_are_finite() {
        let camera = FreeFlyCamera::default();
        let projection =
            perspective_columns(60.0_f32.to_radians(), 16.0 / 9.0, 0.1, 10_000.0).unwrap();
        let view_projection =
            view_projection_columns(&camera, 60.0_f32.to_radians(), 16.0 / 9.0, 0.1, 10_000.0)
                .unwrap();

        assert!(projection.iter().flatten().all(|value| value.is_finite()));
        assert!(
            view_projection
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        );
    }

    #[test]
    fn wider_aspect_reduces_horizontal_projection_scale_only() {
        let square = perspective_columns(PI / 2.0, 1.0, 0.1, 100.0).unwrap();
        let wide = perspective_columns(PI / 2.0, 2.0, 0.1, 100.0).unwrap();

        assert_near(wide[0][0], square[0][0] * 0.5);
        assert_near(wide[1][1], square[1][1]);
    }

    #[test]
    fn narrower_fov_increases_projection_scale() {
        let narrow = perspective_columns(45.0_f32.to_radians(), 1.0, 0.1, 100.0).unwrap();
        let wide = perspective_columns(90.0_f32.to_radians(), 1.0, 0.1, 100.0).unwrap();

        assert!(narrow[0][0] > wide[0][0]);
        assert!(narrow[1][1] > wide[1][1]);
    }

    #[test]
    fn perspective_maps_near_and_far_to_zero_and_one_depth() {
        let near = 0.25;
        let far = 250.0;
        let projection = perspective_columns(PI / 2.0, 1.0, near, far).unwrap();

        let near_clip = transform(projection, [0.0, 0.0, -near, 1.0]);
        let far_clip = transform(projection, [0.0, 0.0, -far, 1.0]);

        assert_near(near_clip[2] / near_clip[3], 0.0);
        assert_near(far_clip[2] / far_clip[3], 1.0);
    }

    #[test]
    fn rejects_invalid_aspect_fov_and_depth_range() {
        assert_eq!(
            perspective_columns(PI / 2.0, 0.0, 0.1, 100.0),
            Err(PerspectiveError::NonPositiveAspectRatio)
        );
        assert_eq!(
            perspective_columns(0.0, 1.0, 0.1, 100.0),
            Err(PerspectiveError::InvalidVerticalFov)
        );
        assert_eq!(
            perspective_columns(PI, 1.0, 0.1, 100.0),
            Err(PerspectiveError::InvalidVerticalFov)
        );
        assert_eq!(
            perspective_columns(PI / 2.0, 1.0, 1.0, 1.0),
            Err(PerspectiveError::InvalidDepthRange)
        );
    }
}
