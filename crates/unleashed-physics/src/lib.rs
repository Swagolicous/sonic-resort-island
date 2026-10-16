#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerTelemetry {
    pub position: glam::Vec3,
    pub velocity: glam::Vec3,
    pub ground_speed: f32,
}
