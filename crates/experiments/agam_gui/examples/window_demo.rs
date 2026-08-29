//! # `window_demo` — Phase 1 Acceptance Desktop Application
//!
//! Interactive Fluent 2D window rendering a background canvas, rounded card
//! container, and reactive clickable accent button.
//!
//! Run with: `cargo run -p agam_gui --example window_demo`

use agam_gui::{
    Color, GpuContext, GpuSurface, GuiApp, GuiEvent, GuiEventLoop, GuiResult, GuiWindow,
    MouseButton, Point, Rect, SceneBuilder, SceneRenderer, WindowConfig,
};

struct DemoApp {
    gpu_context: Option<GpuContext>,
    surface: Option<GpuSurface>,
    renderer: Option<SceneRenderer>,
    cursor: Point,
    clicked: bool,
    dimensions: (u32, u32),
}

impl DemoApp {
    fn new() -> Self {
        Self {
            gpu_context: None,
            surface: None,
            renderer: None,
            cursor: Point::ZERO,
            clicked: false,
            dimensions: (960, 640),
        }
    }

    fn init_gpu(&mut self, window: &GuiWindow) -> GuiResult<()> {
        let context = GpuContext::new()?;
        let surface = context.create_surface(window)?;
        let renderer = SceneRenderer::new(&context)?;

        self.gpu_context = Some(context);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
        Ok(())
    }

    fn render_scene(&mut self) -> GuiResult<()> {
        let (Some(context), Some(surface), Some(renderer)) =
            (&self.gpu_context, &mut self.surface, &mut self.renderer)
        else {
            return Ok(());
        };

        let (width, height) = self.dimensions;
        let mut builder = SceneBuilder::new();

        // 1. Fluent Dark Slate Background
        builder.fill_rect(
            Rect::new(0.0, 0.0, width as f64, height as f64),
            Color::DARK_GRAY,
        );

        // 2. Centered Rounded Card Container
        let card_w = 480.0;
        let card_h = 320.0;
        let card_x = (width as f64 - card_w) / 2.0;
        let card_y = (height as f64 - card_h) / 2.0;

        builder.push_clip_rounded_rect(Rect::new(card_x, card_y, card_w, card_h), 16.0);
        builder.fill_rect(
            Rect::new(card_x, card_y, card_w, card_h),
            Color::rgb(42, 42, 42),
        );

        // 3. Header Accent Strip inside Card
        builder.fill_rect(Rect::new(card_x, card_y, card_w, 6.0), Color::BLUE);

        // 4. Centerpiece 5-Pointed Star
        let star_cx = card_x + card_w / 2.0;
        let star_cy = card_y + 115.0;

        let (star_color, stroke_color, outer_r, inner_r) = if self.clicked {
            (Color::GOLD, Color::WHITE, 56.0, 24.0)
        } else {
            (Color::AMBER, Color::GOLD, 48.0, 20.0)
        };

        // Left & right flanking stars
        builder.fill_star(
            Point::new(star_cx - 120.0, star_cy),
            22.0,
            9.0,
            5,
            Color::rgb(180, 150, 40),
        );
        builder.fill_star(
            Point::new(star_cx + 120.0, star_cy),
            22.0,
            9.0,
            5,
            Color::rgb(180, 150, 40),
        );

        // Main Center Star
        builder.fill_star(
            Point::new(star_cx, star_cy),
            outer_r,
            inner_r,
            5,
            star_color,
        );
        builder.stroke_star(
            Point::new(star_cx, star_cy),
            outer_r,
            inner_r,
            5,
            stroke_color,
            2.0,
        );

        // 5. Interactive Action Button inside Card
        let btn_w = 220.0;
        let btn_h = 44.0;
        let btn_x = card_x + (card_w - btn_w) / 2.0;
        let btn_y = card_y + card_h - 64.0;

        let btn_color = if self.clicked {
            Color::rgb(16, 124, 65) // Fluent Green on active click
        } else {
            Color::rgb(0, 120, 212) // Fluent Blue idle
        };

        builder.fill_rounded_rect(Rect::new(btn_x, btn_y, btn_w, btn_h), 8.0, btn_color);
        builder.pop_clip();

        // Acquire presentation swapchain texture and render frame
        let frame = surface.acquire_frame()?;
        renderer.render_to_frame(context, &builder, &frame, Color::DARK_GRAY)?;
        frame.present();

        Ok(())
    }
}

impl GuiApp for DemoApp {
    fn on_event(&mut self, window: &mut GuiWindow, event: GuiEvent) -> GuiResult<()> {
        // Initialize GPU on first event if not already initialized
        if self.gpu_context.is_none() {
            let _ = self.init_gpu(window);
        }

        match event {
            GuiEvent::PointerMoved { position } => {
                self.cursor = position;
            }
            GuiEvent::PointerDown { button, .. } => {
                if button == MouseButton::Primary {
                    self.clicked = true;
                    window.request_redraw();
                }
            }
            GuiEvent::PointerUp { .. } => {
                if self.clicked {
                    self.clicked = false;
                    window.request_redraw();
                }
            }
            GuiEvent::Resized { width, height, .. } => {
                self.dimensions = (width, height);
                if let Some(ref mut surface) = self.surface {
                    let _ = surface.resize(width, height);
                }
                window.request_redraw();
            }
            GuiEvent::RedrawRequested => {
                self.render_scene()?;
            }
            GuiEvent::CloseRequested => {
                println!("Window close requested — exiting cleanly.");
            }
            _ => {}
        }
        Ok(())
    }
}

fn main() -> GuiResult<()> {
    println!("Launching Agam GUI Engine Phase 1 Acceptance Demo...");
    let event_loop = GuiEventLoop::new()?;
    let config = WindowConfig::new("Agam GUI Engine — Phase 1 Acceptance Demo", 960, 640);
    let app = DemoApp::new();
    event_loop.run(config, app)
}
