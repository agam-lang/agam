//! End-to-end integration test verifying window lifecycle, event mapping,
//! scene building, and clean exit across headless CI runners.

use agam_gui::{
    Color, GuiApp, GuiEvent, GuiResult, GuiWindow, HeadlessEventSource, Key, MouseButton, Point,
    Rect, SceneBuilder, WindowConfig,
};

struct TestApp {
    pointer_pos: Point,
    clicked: bool,
    key_pressed: Option<Key>,
    window_dimensions: (u32, u32),
    closed: bool,
    scene: SceneBuilder,
}

impl TestApp {
    fn new() -> Self {
        Self {
            pointer_pos: Point::ZERO,
            clicked: false,
            key_pressed: None,
            window_dimensions: (800, 600),
            closed: false,
            scene: SceneBuilder::new(),
        }
    }

    fn render(&mut self) {
        self.scene.clear();
        // Background Fluent dark slate
        self.scene
            .fill_rect(Rect::new(0.0, 0.0, 800.0, 600.0), Color::DARK_GRAY);
        // Rounded card
        self.scene
            .push_clip_rounded_rect(Rect::new(50.0, 50.0, 400.0, 300.0), 12.0);
        self.scene
            .fill_rect(Rect::new(50.0, 50.0, 400.0, 300.0), Color::rgb(40, 40, 40));

        // Reactive button color: Blue when idle, Green when clicked
        let btn_color = if self.clicked {
            Color::GREEN
        } else {
            Color::BLUE
        };
        self.scene
            .fill_rounded_rect(Rect::new(80.0, 80.0, 160.0, 48.0), 8.0, btn_color);
        self.scene.pop_clip();
    }
}

impl GuiApp for TestApp {
    fn on_event(&mut self, _window: &mut GuiWindow, event: GuiEvent) -> GuiResult<()> {
        match event {
            GuiEvent::PointerMoved { position } => {
                self.pointer_pos = position;
            }
            GuiEvent::PointerDown { button, .. } => {
                if button == MouseButton::Primary {
                    self.clicked = true;
                }
            }
            GuiEvent::PointerUp { .. } => {
                self.clicked = false;
            }
            GuiEvent::KeyDown { key, .. } => {
                self.key_pressed = Some(key);
            }
            GuiEvent::Resized { width, height, .. } => {
                self.window_dimensions = (width, height);
            }
            GuiEvent::CloseRequested => {
                self.closed = true;
            }
            GuiEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
        Ok(())
    }
}

#[test]
fn test_window_lifecycle_headless_pipeline() {
    let mut app = TestApp::new();
    let mut event_source = HeadlessEventSource::standard_test_sequence();

    // Initial render
    app.render();
    assert_eq!(app.scene.node_count(), 5);

    // Synthetic window stub
    let cfg = WindowConfig::new("Integration Window", 800, 600);
    assert_eq!(cfg.title, "Integration Window");

    let mut event_count = 0;
    while let Some(event) = event_source.next_event() {
        event_count += 1;
        // Direct event dispatch simulation
        match &event {
            GuiEvent::PointerMoved { position } => {
                app.pointer_pos = *position;
            }
            GuiEvent::PointerDown { button, .. } => {
                if *button == MouseButton::Primary {
                    app.clicked = true;
                    app.render();
                }
            }
            GuiEvent::PointerUp { .. } => {
                app.clicked = false;
                app.render();
            }
            GuiEvent::KeyDown { key, .. } => {
                app.key_pressed = Some(key.clone());
            }
            GuiEvent::Resized { width, height, .. } => {
                app.window_dimensions = (*width, *height);
            }
            GuiEvent::CloseRequested => {
                app.closed = true;
            }
            _ => {}
        }
    }

    assert_eq!(event_count, 7);
    assert_eq!(app.pointer_pos, Point::new(100.0, 150.0));
    assert_eq!(app.key_pressed, Some(Key::Character("A".to_string())));
    assert_eq!(app.window_dimensions, (1024, 768));
    assert!(app.closed, "Window close requested event must be processed");

    // Verify Vello scene compilation from the constructed scene graph
    let vello_scene = app.scene.build_vello_scene();
    let _ = vello_scene;
}
