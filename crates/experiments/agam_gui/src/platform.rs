//! Platform windowing, lifecycle, and OS event translation facade.
//!
//! Under the Agam zero-identity-leak invariant, all `winit` types and event loops
//! are strictly confined within this module and never leaked to scripts or callers.

use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{
    ElementState as WinitElementState, MouseButton as WinitMouseButton,
    WindowEvent as WinitWindowEvent,
};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window as WinitWindow, WindowAttributes, WindowId};

use crate::diagnostic::{GuiError, GuiResult};
use crate::input::{GuiEvent, Key, Modifiers, MouseButton};
use crate::scene::Point;

/// Configuration for creating a native Agam GUI window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title shown on native title bar.
    pub title: String,
    /// Logical window width in pixels.
    pub width: u32,
    /// Logical window height in pixels.
    pub height: u32,
    /// Whether the window is resizable by the user.
    pub resizable: bool,
    /// Whether the window has OS titlebar and borders.
    pub decorations: bool,
    /// Whether the window starts visible.
    pub visible: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Agam Application".to_string(),
            width: 800,
            height: 600,
            resizable: true,
            decorations: true,
            visible: true,
        }
    }
}

impl WindowConfig {
    /// Create a new window config with custom title and dimensions.
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
            ..Default::default()
        }
    }

    /// Build `winit::window::WindowAttributes` from this config.
    #[allow(dead_code)]
    pub(crate) fn to_window_attributes(&self) -> WindowAttributes {
        WindowAttributes::default()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(self.width, self.height))
            .with_resizable(self.resizable)
            .with_decorations(self.decorations)
            .with_visible(self.visible)
    }
}

/// An Agam-managed native window handle.
///
/// Encapsulates the underlying OS window without leaking `winit` types.
#[derive(Clone)]
pub struct GuiWindow {
    window: Arc<WinitWindow>,
    title: String,
}

impl GuiWindow {
    /// Wrap an internal `winit::window::Window`.
    #[allow(dead_code)]
    pub(crate) fn new(window: Arc<WinitWindow>, title: String) -> Self {
        Self { window, title }
    }

    /// Return a numeric handle identifier for this window.
    pub fn id(&self) -> u64 {
        let raw_id: WindowId = self.window.id();
        // Extract deterministic numeric ID from WindowId
        let id_u64: u64 = unsafe { std::mem::transmute_copy(&raw_id) };
        id_u64
    }

    /// Get current window title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set window title.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        self.window.set_title(title);
    }

    /// Get inner physical dimensions `(width, height)` in pixels.
    pub fn inner_size(&self) -> (u32, u32) {
        let size: PhysicalSize<u32> = self.window.inner_size();
        (size.width, size.height)
    }

    /// Get inner logical dimensions `(width, height)` in points.
    pub fn logical_size(&self) -> (f64, f64) {
        let physical = self.window.inner_size();
        let scale = self.window.scale_factor();
        (
            physical.width as f64 / scale,
            physical.height as f64 / scale,
        )
    }

    /// Get current display scale factor (DPI multiplier).
    pub fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    /// Request a redraw event for this window on the next vsync/frame cycle.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Set window visibility (e.g. show after first frame is presented to prevent blank white flashes).
    pub fn set_visible(&self, visible: bool) {
        self.window.set_visible(visible);
    }

    /// Provide internal access to the underlying `winit::window::Window` for `wgpu` surface creation.
    #[allow(dead_code)]
    pub(crate) fn raw_window(&self) -> &Arc<WinitWindow> {
        &self.window
    }
}

/// Convert an OS window creation failure into a structured Nyāya `GuiError`.
pub fn map_os_error(err: &impl std::fmt::Display) -> GuiError {
    GuiError::new(
        format!("Failed to create platform window: {err}"),
        "Host windowing system rejected window creation or configuration attributes",
        Some("Verify display server availability (X11/Wayland/Win32) or enable headless-test mode"),
        "RFC-gui-engine §1: Windowing errors must not panic and must return structured Nyāya diagnostics",
    )
}

/// Translate a raw `winit::event::WindowEvent` into an Agam-owned `GuiEvent`.
///
/// Returns `None` if the event is internal/ignored by the Agam event pipeline.
pub fn map_window_event(event: &WinitWindowEvent, last_cursor_pos: &mut Point) -> Option<GuiEvent> {
    match event {
        WinitWindowEvent::Resized(size) => Some(GuiEvent::Resized {
            width: size.width,
            height: size.height,
            scale_factor: 1.0,
        }),
        WinitWindowEvent::CloseRequested => Some(GuiEvent::CloseRequested),
        WinitWindowEvent::Focused(focused) => Some(GuiEvent::FocusChanged(*focused)),
        WinitWindowEvent::CursorMoved { position, .. } => {
            let pt = Point::new(position.x, position.y);
            *last_cursor_pos = pt;
            Some(GuiEvent::PointerMoved { position: pt })
        }
        WinitWindowEvent::MouseInput { state, button, .. } => {
            let mapped_button = match button {
                WinitMouseButton::Left => MouseButton::Primary,
                WinitMouseButton::Right => MouseButton::Secondary,
                WinitMouseButton::Middle => MouseButton::Middle,
                WinitMouseButton::Back => MouseButton::Back,
                WinitMouseButton::Forward => MouseButton::Forward,
                WinitMouseButton::Other(code) => MouseButton::Other(*code),
            };
            match state {
                WinitElementState::Pressed => Some(GuiEvent::PointerDown {
                    button: mapped_button,
                    position: *last_cursor_pos,
                }),
                WinitElementState::Released => Some(GuiEvent::PointerUp {
                    button: mapped_button,
                    position: *last_cursor_pos,
                }),
            }
        }
        WinitWindowEvent::MouseWheel { delta, .. } => {
            let (dx, dy) = match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => (*x as f64, *y as f64),
                winit::event::MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => (*x, *y),
            };
            Some(GuiEvent::MouseWheel {
                delta_x: dx,
                delta_y: dy,
            })
        }
        WinitWindowEvent::KeyboardInput { event, .. } => {
            let mapped_key = match &event.logical_key {
                WinitKey::Named(named) => map_named_key(named),
                WinitKey::Character(s) => Key::Character(s.to_string()),
                WinitKey::Unidentified(u) => Key::Other(format!("{u:?}")),
                WinitKey::Dead(d) => Key::Other(format!("DeadKey({d:?})")),
            };
            match event.state {
                WinitElementState::Pressed => Some(GuiEvent::KeyDown {
                    key: mapped_key,
                    text: event.text.as_ref().map(|s| s.to_string()),
                    repeat: event.repeat,
                }),
                WinitElementState::Released => Some(GuiEvent::KeyUp { key: mapped_key }),
            }
        }
        WinitWindowEvent::ModifiersChanged(modifiers) => {
            let state = modifiers.state();
            Some(GuiEvent::ModifiersChanged(Modifiers {
                shift: state.shift_key(),
                ctrl: state.control_key(),
                alt: state.alt_key(),
                meta: state.super_key(),
            }))
        }
        WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            Some(GuiEvent::ScaleFactorChanged {
                scale_factor: *scale_factor,
            })
        }
        WinitWindowEvent::RedrawRequested => Some(GuiEvent::RedrawRequested),
        _ => None,
    }
}

/// Map a `winit::keyboard::NamedKey` to an Agam `Key`.
fn map_named_key(named: &NamedKey) -> Key {
    match named {
        NamedKey::Escape => Key::Escape,
        NamedKey::Enter => Key::Enter,
        NamedKey::Tab => Key::Tab,
        NamedKey::Space => Key::Space,
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Delete => Key::Delete,
        NamedKey::ArrowUp => Key::ArrowUp,
        NamedKey::ArrowDown => Key::ArrowDown,
        NamedKey::ArrowLeft => Key::ArrowLeft,
        NamedKey::ArrowRight => Key::ArrowRight,
        NamedKey::Home => Key::Home,
        NamedKey::End => Key::End,
        NamedKey::PageUp => Key::PageUp,
        NamedKey::PageDown => Key::PageDown,
        NamedKey::Insert => Key::Insert,
        NamedKey::F1 => Key::F(1),
        NamedKey::F2 => Key::F(2),
        NamedKey::F3 => Key::F(3),
        NamedKey::F4 => Key::F(4),
        NamedKey::F5 => Key::F(5),
        NamedKey::F6 => Key::F(6),
        NamedKey::F7 => Key::F(7),
        NamedKey::F8 => Key::F(8),
        NamedKey::F9 => Key::F(9),
        NamedKey::F10 => Key::F(10),
        NamedKey::F11 => Key::F(11),
        NamedKey::F12 => Key::F(12),
        other => Key::Other(format!("{other:?}")),
    }
}

/// Application lifecycle trait for Agam GUI apps.
pub trait GuiApp {
    /// Invoked on every mapped `GuiEvent`.
    fn on_event(&mut self, window: &mut GuiWindow, event: GuiEvent) -> GuiResult<()>;
}

/// Central application event loop for Agam GUI applications.
pub struct GuiEventLoop {
    event_loop: EventLoop<()>,
}

impl GuiEventLoop {
    /// Create a new platform event loop.
    pub fn new() -> GuiResult<Self> {
        let event_loop = EventLoop::new().map_err(|e| map_os_error(&e))?;
        Ok(Self { event_loop })
    }

    /// Run the application event loop with the given initial window configuration.
    pub fn run<A: GuiApp + 'static>(self, config: WindowConfig, app: A) -> GuiResult<()> {
        let mut runner = AppRunner {
            config,
            app,
            window: None,
            cursor: Point::new(0.0, 0.0),
        };
        self.event_loop
            .run_app(&mut runner)
            .map_err(|e| map_os_error(&e))
    }
}

struct AppRunner<A: GuiApp> {
    config: WindowConfig,
    app: A,
    window: Option<GuiWindow>,
    cursor: Point,
}

impl<A: GuiApp> ApplicationHandler for AppRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = self.config.to_window_attributes();
            match event_loop.create_window(attrs) {
                Ok(w) => {
                    let mut gui_window = GuiWindow::new(Arc::new(w), self.config.title.clone());
                    // Render first frame immediately to eliminate white startup latency
                    let _ = self
                        .app
                        .on_event(&mut gui_window, GuiEvent::RedrawRequested);
                    self.window = Some(gui_window);
                }
                Err(err) => {
                    eprintln!("Failed to create platform window: {err}");
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        let (Some(window), Some(gui_event)) =
            (&mut self.window, map_window_event(&event, &mut self.cursor))
        else {
            return;
        };

        if let GuiEvent::CloseRequested = gui_event {
            let _ = self.app.on_event(window, gui_event);
            event_loop.exit();
        } else {
            let _ = self.app.on_event(window, gui_event);
        }
    }
}

/// Synthetic event generator for headless CI and unit testing.
pub struct HeadlessEventSource {
    events: Vec<GuiEvent>,
    cursor: usize,
}

impl HeadlessEventSource {
    /// Create a new headless event source with an initial sequence of synthetic events.
    pub fn new(events: Vec<GuiEvent>) -> Self {
        Self { events, cursor: 0 }
    }

    /// Produce standard test event sequence (Pointer move, click, keydown, resize, close).
    pub fn standard_test_sequence() -> Self {
        Self::new(vec![
            GuiEvent::PointerMoved {
                position: Point::new(100.0, 150.0),
            },
            GuiEvent::PointerDown {
                button: MouseButton::Primary,
                position: Point::new(100.0, 150.0),
            },
            GuiEvent::PointerUp {
                button: MouseButton::Primary,
                position: Point::new(100.0, 150.0),
            },
            GuiEvent::KeyDown {
                key: Key::Character("A".to_string()),
                text: Some("A".to_string()),
                repeat: false,
            },
            GuiEvent::KeyUp {
                key: Key::Character("A".to_string()),
            },
            GuiEvent::Resized {
                width: 1024,
                height: 768,
                scale_factor: 1.0,
            },
            GuiEvent::CloseRequested,
        ])
    }

    /// Poll next event from synthetic stream.
    pub fn next_event(&mut self) -> Option<GuiEvent> {
        if self.cursor < self.events.len() {
            let ev = self.events[self.cursor].clone();
            self.cursor += 1;
            Some(ev)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_defaults() {
        let cfg = WindowConfig::default();
        assert_eq!(cfg.title, "Agam Application");
        assert_eq!(cfg.width, 800);
        assert_eq!(cfg.height, 600);
        assert!(cfg.resizable);
        assert!(cfg.decorations);
        assert!(cfg.visible);

        let custom = WindowConfig::new("Editor", 1280, 720);
        assert_eq!(custom.title, "Editor");
        assert_eq!(custom.width, 1280);
        assert_eq!(custom.height, 720);
    }

    #[test]
    fn test_map_named_keys() {
        assert_eq!(map_named_key(&NamedKey::Escape), Key::Escape);
        assert_eq!(map_named_key(&NamedKey::Enter), Key::Enter);
        assert_eq!(map_named_key(&NamedKey::Tab), Key::Tab);
        assert_eq!(map_named_key(&NamedKey::Space), Key::Space);
        assert_eq!(map_named_key(&NamedKey::ArrowDown), Key::ArrowDown);
        assert_eq!(map_named_key(&NamedKey::F5), Key::F(5));
    }

    #[test]
    fn test_map_window_event_pointer_lifecycle() {
        let mut cursor = Point::new(0.0, 0.0);

        // 1. Move cursor
        let move_ev = WinitWindowEvent::CursorMoved {
            device_id: unsafe { std::mem::zeroed() },
            position: PhysicalPosition::new(320.0, 240.0),
        };
        let mapped = map_window_event(&move_ev, &mut cursor);
        assert_eq!(
            mapped,
            Some(GuiEvent::PointerMoved {
                position: Point::new(320.0, 240.0)
            })
        );
        assert_eq!(cursor, Point::new(320.0, 240.0));

        // 2. Mouse click down
        let down_ev = WinitWindowEvent::MouseInput {
            device_id: unsafe { std::mem::zeroed() },
            state: WinitElementState::Pressed,
            button: WinitMouseButton::Left,
        };
        let mapped_down = map_window_event(&down_ev, &mut cursor);
        assert_eq!(
            mapped_down,
            Some(GuiEvent::PointerDown {
                button: MouseButton::Primary,
                position: Point::new(320.0, 240.0),
            })
        );

        // 3. Mouse click up
        let up_ev = WinitWindowEvent::MouseInput {
            device_id: unsafe { std::mem::zeroed() },
            state: WinitElementState::Released,
            button: WinitMouseButton::Left,
        };
        let mapped_up = map_window_event(&up_ev, &mut cursor);
        assert_eq!(
            mapped_up,
            Some(GuiEvent::PointerUp {
                button: MouseButton::Primary,
                position: Point::new(320.0, 240.0),
            })
        );
    }

    #[test]
    fn test_map_window_event_resize_and_close() {
        let mut cursor = Point::new(0.0, 0.0);

        let resize_ev = WinitWindowEvent::Resized(PhysicalSize::new(1920, 1080));
        let mapped_resize = map_window_event(&resize_ev, &mut cursor);
        assert_eq!(
            mapped_resize,
            Some(GuiEvent::Resized {
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
            })
        );

        let close_ev = WinitWindowEvent::CloseRequested;
        assert_eq!(
            map_window_event(&close_ev, &mut cursor),
            Some(GuiEvent::CloseRequested)
        );

        let focus_ev = WinitWindowEvent::Focused(true);
        assert_eq!(
            map_window_event(&focus_ev, &mut cursor),
            Some(GuiEvent::FocusChanged(true))
        );
    }

    #[test]
    fn test_headless_event_source() {
        let mut src = HeadlessEventSource::standard_test_sequence();
        let mut count = 0;
        while let Some(_ev) = src.next_event() {
            count += 1;
        }
        assert_eq!(count, 7);
    }

    #[test]
    fn test_map_os_error_nyaya_proof() {
        let err = map_os_error(&"Wayland compositor unavailable");
        assert!(err.fact.contains("Wayland compositor unavailable"));
        assert!(err.fix.is_some());
        let proof = err.to_proof();
        assert_eq!(proof.fact, err.fact);
    }
}
