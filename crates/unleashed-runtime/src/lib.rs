pub mod camera;
pub mod debug;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use camera::{FreeFlyCamera, FreeFlyInput};
use unleashed_platform::{FrameDriver, InputSnapshot};

const DEVELOPMENT_CRATE_ROOT: &str = "unleashed-rs";
const WINDMILL_ISLE_NAME: &str = "Windmill Isle Act 1";
const WINDMILL_ISLE_ARCHIVE_STEM: &str = "ActD_MykonosAct1";
const WINDMILL_ISLE_SPAWN: [f32; 3] = [391.55, 44.48, 104.08];
const WINDMILL_ISLE_SPAWN_YAW_DEGREES: f32 = -120.0;
const DEBUG_CAMERA_VERTICAL_FOV_DEGREES: f32 = 60.0;
const DEBUG_CAMERA_NEAR_PLANE: f32 = 0.1;
const DEBUG_CAMERA_FAR_PLANE: f32 = 10_000.0;
const INITIAL_WINDOW_WIDTH: u32 = 1280;
const INITIAL_WINDOW_HEIGHT: u32 = 720;

/// Runtime-owned state prepared before the platform creates its window.
///
/// Stage decoding is intentionally not performed here yet: `unleashed-runtime`
/// does not currently depend on `unleashed-assets`, and that crate does not
/// export a stage-loader facade. Keeping bootstrap state independent of those
/// APIs lets the runtime compile while the loader boundary is being completed.
#[derive(Debug)]
struct RuntimeBootstrap {
    game_root: PathBuf,
    camera: FreeFlyCamera,
}

impl RuntimeBootstrap {
    fn discover() -> Result<Self> {
        let executable = std::env::current_exe().context("locate current executable")?;
        let game_root = locate_development_game_root(&executable).with_context(|| {
            format!(
                "locate game root from executable {} (expected an `{DEVELOPMENT_CRATE_ROOT}` ancestor)",
                executable.display()
            )
        })?;

        let mut camera = FreeFlyCamera::new([
            WINDMILL_ISLE_SPAWN[0],
            WINDMILL_ISLE_SPAWN[1] + 3.0,
            WINDMILL_ISLE_SPAWN[2] + 8.0,
        ]);
        camera.set_pose(
            camera.position,
            WINDMILL_ISLE_SPAWN_YAW_DEGREES.to_radians(),
            -0.15,
        );

        Ok(Self { game_root, camera })
    }

    fn print_diagnostics(&self) {
        eprintln!("[runtime] root  {}", self.game_root.display());
        eprintln!("[runtime] stage {WINDMILL_ISLE_NAME} ({WINDMILL_ISLE_ARCHIVE_STEM})");
        eprintln!(
            "[runtime] camera [{:.2}, {:.2}, {:.2}] yaw {:.1}deg",
            self.camera.position[0],
            self.camera.position[1],
            self.camera.position[2],
            self.camera.yaw.to_degrees()
        );
        eprintln!(
            "[runtime] assets deferred: unleashed-assets exports parsers, but no stage-loader facade"
        );
        eprintln!("[runtime] terrain deferred: Renderer has no decoded-mesh upload/ownership API");
    }
}

/// Finds the development game root without depending on the process working
/// directory.
///
/// Development executables normally live below `.../unleashed-rs/target/...`;
/// the original game-data roots (`game`, `update`, `dlc`) live beside
/// `unleashed-rs`, so the desired root is the parent of that ancestor.
fn locate_development_game_root(executable: &Path) -> Option<PathBuf> {
    executable
        .ancestors()
        .find(|ancestor| {
            ancestor
                .file_name()
                .is_some_and(|name| name == DEVELOPMENT_CRATE_ROOT)
        })
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

/// Owns orchestration state that must survive for the full render session.
///
/// The camera is retained here now so input/render integration can consume it
/// once the platform forwards input and `Renderer` exposes camera updates.
struct RuntimeDriver {
    renderer: unleashed_render::Renderer,
    camera: FreeFlyCamera,
    viewport_width: u32,
    viewport_height: u32,
}

impl RuntimeDriver {
    fn update_camera_matrix(&mut self) -> Result<()> {
        if self.viewport_width == 0 || self.viewport_height == 0 {
            return Ok(());
        }

        let aspect_ratio = self.viewport_width as f32 / self.viewport_height as f32;
        let view_projection = camera::math::view_projection_columns(
            &self.camera,
            DEBUG_CAMERA_VERTICAL_FOV_DEGREES.to_radians(),
            aspect_ratio,
            DEBUG_CAMERA_NEAR_PLANE,
            DEBUG_CAMERA_FAR_PLANE,
        )
        .context("build debug camera view-projection matrix")?;

        self.renderer.update_terrain_camera(view_projection);
        Ok(())
    }
}

impl FrameDriver for RuntimeDriver {
    fn resize(&mut self, width: u32, height: u32) {
        self.viewport_width = width;
        self.viewport_height = height;
        self.renderer.resize(width, height);
    }

    fn update(&mut self, input: &InputSnapshot, delta: std::time::Duration) -> Result<()> {
        let movement = [
            (input.right as i8 - input.left as i8) as f32,
            (input.up as i8 - input.ctrl as i8) as f32,
            (input.forward as i8 - input.backward as i8) as f32,
        ];

        self.camera.update(
            delta.as_secs_f32(),
            FreeFlyInput {
                movement,
                look_delta: [input.mouse_delta.0 as f32, -input.mouse_delta.1 as f32],
                boost: input.shift,
            },
        );

        self.update_camera_matrix()
    }

    fn render(&mut self) -> Result<()> {
        self.renderer.render()
    }
}

pub fn run() -> Result<()> {
    let bootstrap = RuntimeBootstrap::discover()?;
    bootstrap.print_diagnostics();

    let camera = bootstrap.camera;
    unleashed_platform::run(move |window| {
        Ok(RuntimeDriver {
            renderer: unleashed_render::Renderer::new(window)?,
            camera,
            viewport_width: INITIAL_WINDOW_WIDTH,
            viewport_height: INITIAL_WINDOW_HEIGHT,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locates_parent_of_unleashed_rs_from_development_executable() {
        let executable =
            Path::new(r"D:\UnleashedRecomp-Windows\unleashed-rs\target\debug\unleashed.exe");

        assert_eq!(
            locate_development_game_root(executable),
            Some(PathBuf::from(r"D:\UnleashedRecomp-Windows"))
        );
    }

    #[test]
    fn rejects_executable_without_unleashed_rs_ancestor() {
        let executable = Path::new(r"D:\elsewhere\target\debug\unleashed.exe");
        assert_eq!(locate_development_game_root(executable), None);
    }
}
