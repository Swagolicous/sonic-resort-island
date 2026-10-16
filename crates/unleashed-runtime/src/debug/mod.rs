//! Low-overhead runtime telemetry for stage/debug inspection.
//!
//! Per-frame updates are allocation-free. Text formatting allocates only when
//! [`Telemetry::text_snapshot`] is explicitly requested.

use std::fmt::Write;
use std::time::Duration;

pub const DEFAULT_FRAME_WINDOW: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameTimingSnapshot {
    /// Latest complete frame duration.
    pub frame_time_ms: f32,
    /// Reciprocal of the latest complete frame duration.
    pub fps: f32,
    /// Mean duration across the populated rolling window.
    pub average_frame_time_ms: f32,
    /// FPS derived from the rolling mean frame duration.
    pub average_fps: f32,
    pub min_frame_time_ms: f32,
    pub max_frame_time_ms: f32,
    pub sample_count: usize,
}

impl Default for FrameTimingSnapshot {
    fn default() -> Self {
        Self {
            frame_time_ms: 0.0,
            fps: 0.0,
            average_frame_time_ms: 0.0,
            average_fps: 0.0,
            min_frame_time_ms: 0.0,
            max_frame_time_ms: 0.0,
            sample_count: 0,
        }
    }
}

/// Fixed-capacity rolling frame timing accumulator.
///
/// No heap allocation occurs while samples are recorded.
#[derive(Debug, Clone)]
pub struct FrameTimingWindow<const N: usize = DEFAULT_FRAME_WINDOW> {
    samples_ms: [f32; N],
    next: usize,
    count: usize,
    sum_ms: f32,
    latest_ms: f32,
}

impl<const N: usize> Default for FrameTimingWindow<N> {
    fn default() -> Self {
        assert!(N > 0, "frame timing window capacity must be non-zero");
        Self {
            samples_ms: [0.0; N],
            next: 0,
            count: 0,
            sum_ms: 0.0,
            latest_ms: 0.0,
        }
    }
}

impl<const N: usize> FrameTimingWindow<N> {
    pub fn push_duration(&mut self, duration: Duration) {
        self.push_seconds(duration.as_secs_f32());
    }

    pub fn push_seconds(&mut self, seconds: f32) {
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }

        let frame_ms = seconds * 1000.0;
        self.latest_ms = frame_ms;

        if self.count == N {
            self.sum_ms -= self.samples_ms[self.next];
        } else {
            self.count += 1;
        }

        self.samples_ms[self.next] = frame_ms;
        self.sum_ms += frame_ms;
        self.next = (self.next + 1) % N;
    }

    pub fn clear(&mut self) {
        self.samples_ms.fill(0.0);
        self.next = 0;
        self.count = 0;
        self.sum_ms = 0.0;
        self.latest_ms = 0.0;
    }

    pub fn snapshot(&self) -> FrameTimingSnapshot {
        if self.count == 0 {
            return FrameTimingSnapshot::default();
        }

        let mut min_ms = f32::INFINITY;
        let mut max_ms = 0.0_f32;
        for &sample in &self.samples_ms[..self.count] {
            min_ms = min_ms.min(sample);
            max_ms = max_ms.max(sample);
        }

        let average_ms = self.sum_ms / self.count as f32;

        FrameTimingSnapshot {
            frame_time_ms: self.latest_ms,
            fps: fps_from_ms(self.latest_ms),
            average_frame_time_ms: average_ms,
            average_fps: fps_from_ms(average_ms),
            min_frame_time_ms: min_ms,
            max_frame_time_ms: max_ms,
            sample_count: self.count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerSnapshot {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub ground_speed: f32,
    /// Stable numeric game state identifier when available.
    pub state_id: u32,
    /// Optional human-readable state name from static runtime metadata.
    pub state_name: Option<&'static str>,
}

impl Default for PlayerSnapshot {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            velocity: [0.0; 3],
            ground_speed: 0.0,
            state_id: 0,
            state_name: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderSnapshot {
    pub draw_calls: u32,
    pub loaded_chunks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TelemetrySnapshot {
    pub timing: FrameTimingSnapshot,
    pub player: PlayerSnapshot,
    pub render: RenderSnapshot,
}

/// Mutable collection point intended to live alongside runtime state.
///
/// Call [`record_frame`](Self::record_frame) once per completed frame, update
/// player/render counters when their source data is available, and request a
/// copied snapshot or text only when debug tooling needs it.
#[derive(Debug, Clone)]
pub struct Telemetry<const N: usize = DEFAULT_FRAME_WINDOW> {
    timing: FrameTimingWindow<N>,
    player: PlayerSnapshot,
    render: RenderSnapshot,
}

impl<const N: usize> Default for Telemetry<N> {
    fn default() -> Self {
        Self {
            timing: FrameTimingWindow::default(),
            player: PlayerSnapshot::default(),
            render: RenderSnapshot::default(),
        }
    }
}

impl<const N: usize> Telemetry<N> {
    pub fn record_frame(&mut self, duration: Duration) {
        self.timing.push_duration(duration);
    }

    pub fn record_frame_seconds(&mut self, seconds: f32) {
        self.timing.push_seconds(seconds);
    }

    pub fn set_player(
        &mut self,
        position: [f32; 3],
        velocity: [f32; 3],
        ground_speed: f32,
        state_id: u32,
        state_name: Option<&'static str>,
    ) {
        self.player = PlayerSnapshot {
            position,
            velocity,
            ground_speed,
            state_id,
            state_name,
        };
    }

    pub fn set_render_stats(&mut self, draw_calls: u32, loaded_chunks: u32) {
        self.render = RenderSnapshot {
            draw_calls,
            loaded_chunks,
        };
    }

    pub fn snapshot(&self) -> TelemetrySnapshot {
        TelemetrySnapshot {
            timing: self.timing.snapshot(),
            player: self.player,
            render: self.render,
        }
    }

    /// Produces a compact multi-line debug snapshot suitable for logging or a
    /// future UI text widget.
    pub fn text_snapshot(&self) -> String {
        self.snapshot().to_text()
    }
}

impl TelemetrySnapshot {
    pub fn to_text(self) -> String {
        let mut text = String::with_capacity(320);
        let timing = self.timing;
        let player = self.player;

        let _ = writeln!(
            text,
            "FPS {:>6.1} | Frame {:>6.2} ms | Avg {:>6.1} FPS ({:>6.2} ms) | Range {:>6.2}-{:>6.2} ms [{}]",
            timing.fps,
            timing.frame_time_ms,
            timing.average_fps,
            timing.average_frame_time_ms,
            timing.min_frame_time_ms,
            timing.max_frame_time_ms,
            timing.sample_count,
        );
        let _ = writeln!(
            text,
            "Player pos [{:>8.2}, {:>8.2}, {:>8.2}] | vel [{:>8.2}, {:>8.2}, {:>8.2}] | ground {:>7.2}",
            player.position[0],
            player.position[1],
            player.position[2],
            player.velocity[0],
            player.velocity[1],
            player.velocity[2],
            player.ground_speed,
        );

        match player.state_name {
            Some(name) => {
                let _ = writeln!(text, "State {} ({})", name, player.state_id);
            }
            None => {
                let _ = writeln!(text, "State {}", player.state_id);
            }
        }

        let _ = write!(
            text,
            "Draw calls {} | Loaded chunks {}",
            self.render.draw_calls, self.render.loaded_chunks
        );

        text
    }
}

fn fps_from_ms(frame_ms: f32) -> f32 {
    if frame_ms > 0.0 && frame_ms.is_finite() {
        1000.0 / frame_ms
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_window_evicts_old_samples() {
        let mut timing = FrameTimingWindow::<2>::default();
        timing.push_seconds(0.010);
        timing.push_seconds(0.020);
        timing.push_seconds(0.030);

        let snapshot = timing.snapshot();
        assert_eq!(snapshot.sample_count, 2);
        assert!((snapshot.average_frame_time_ms - 25.0).abs() < 1.0e-5);
        assert!((snapshot.min_frame_time_ms - 20.0).abs() < 1.0e-5);
        assert!((snapshot.max_frame_time_ms - 30.0).abs() < 1.0e-5);
    }

    #[test]
    fn invalid_samples_are_ignored() {
        let mut timing = FrameTimingWindow::<4>::default();
        timing.push_seconds(0.0);
        timing.push_seconds(f32::NAN);
        assert_eq!(timing.snapshot().sample_count, 0);
    }

    #[test]
    fn text_snapshot_contains_requested_runtime_fields() {
        let mut telemetry = Telemetry::<4>::default();
        telemetry.record_frame_seconds(1.0 / 60.0);
        telemetry.set_player([1.0, 2.0, 3.0], [4.0, 5.0, 6.0], 7.0, 42, Some("Running"));
        telemetry.set_render_stats(123, 9);

        let text = telemetry.text_snapshot();
        assert!(text.contains("FPS"));
        assert!(text.contains("Running"));
        assert!(text.contains("Draw calls 123"));
        assert!(text.contains("Loaded chunks 9"));
    }
}
