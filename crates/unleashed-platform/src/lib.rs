use anyhow::Result;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{WindowAttributes, WindowId},
};

pub use winit::window::Window;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputSnapshot {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub mouse_delta: (f64, f64),
    pub focused: bool,
}

impl InputSnapshot {
    fn set_key(&mut self, code: KeyCode, held: bool) {
        match code {
            KeyCode::KeyW => self.forward = held,
            KeyCode::KeyS => self.backward = held,
            KeyCode::KeyA => self.left = held,
            KeyCode::KeyD => self.right = held,
            KeyCode::Space => self.up = held,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.shift = held,
            KeyCode::ControlLeft | KeyCode::ControlRight => self.ctrl = held,
            _ => {}
        }
    }

    fn clear_held(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.up = false;
        self.shift = false;
        self.ctrl = false;
    }

    fn clear_frame_delta(&mut self) {
        self.mouse_delta = (0.0, 0.0);
    }
}

pub trait FrameDriver: 'static {
    fn resize(&mut self, width: u32, height: u32);

    fn update(&mut self, _input: &InputSnapshot, _delta: Duration) -> Result<()> {
        Ok(())
    }

    fn render(&mut self) -> Result<()>;
}

struct App<F, D>
where
    F: FnOnce(Window) -> Result<D>,
    D: FrameDriver,
{
    create_driver: Option<F>,
    driver: Option<D>,
    window_id: Option<WindowId>,
    input: InputSnapshot,
    last_frame: Option<Instant>,
}

impl<F, D> ApplicationHandler for App<F, D>
where
    F: FnOnce(Window) -> Result<D> + 'static,
    D: FrameDriver,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.driver.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Sonic Unleashed — Rust Reconstruction")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));

        let window = event_loop.create_window(attrs).expect("create window");
        self.window_id = Some(window.id());
        let create = self.create_driver.take().expect("driver factory");
        self.driver = Some(create(window).expect("initialize renderer"));
        self.last_frame = Some(Instant::now());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => {
                self.input.focused = focused;
                if !focused {
                    self.input.clear_held();
                    self.input.clear_frame_delta();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let held = event.state == ElementState::Pressed;

                    if code == KeyCode::Escape && held {
                        event_loop.exit();
                        return;
                    }

                    self.input.set_key(code, held);
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(driver) = self.driver.as_mut() {
                    driver.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta = self
                    .last_frame
                    .replace(now)
                    .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));

                let input = self.input;
                self.input.clear_frame_delta();

                if let Some(driver) = self.driver.as_mut() {
                    if let Err(err) = driver.update(&input, delta) {
                        eprintln!("update error: {err:#}");
                    }

                    if let Err(err) = driver.render() {
                        eprintln!("render error: {err:#}");
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if !self.input.focused {
            return;
        }

        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.mouse_delta.0 += delta.0;
            self.input.mouse_delta.1 += delta.1;
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // The renderer owns the window and requests redraw after each frame.
    }
}

pub fn run<F, D>(create_driver: F) -> Result<()>
where
    F: FnOnce(Window) -> Result<D> + 'static,
    D: FrameDriver,
{
    let event_loop = EventLoop::new()?;
    let mut app = App {
        create_driver: Some(create_driver),
        driver: None,
        window_id: None,
        input: InputSnapshot::default(),
        last_frame: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
