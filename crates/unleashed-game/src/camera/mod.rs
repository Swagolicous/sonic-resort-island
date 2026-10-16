//! Sonic's cameras (SWA::CPlayer3DNormalCamera3 for 3D, CPlayer2DNormalCameraNew
//! for 2D), ported from the recompiled executable. Parameters come from the
//! player's own `Camera.prm.xml`; see sheets/camera_fields.csv and
//! sheets/port_functions.csv for what is ported.

pub mod params;

pub use params::{CommonCameraParams, ForwardCameraParams, SideCameraParams};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_path_and_reports_missing_ones() {
        let all = |p: &str| ForwardCameraParams::PATHS.contains(&p).then_some(if p.ends_with("s_IsDebugDraw") { "false" } else { "1.5" });
        let p = ForwardCameraParams::read(all).expect("all paths present");
        assert_eq!(p.distance, 1.5);
        let missing = ForwardCameraParams::read(|p| (p != "Sonic/Forward/Basic/param.Distance").then_some("2"));
        assert!(missing.unwrap_err().contains(&"Sonic/Forward/Basic/param.Distance"));
    }
}
