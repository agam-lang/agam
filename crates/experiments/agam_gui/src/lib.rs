//! # `agam_gui` — Native GPU-Accelerated Declarative GUI Engine
//!
//! Provides a retained 2D vector rendering pipeline, platform windowing facade,
//! and Nyāya-grounded diagnostics adhering to the zero-identity-leak boundary invariant.

pub mod apps;
pub mod diagnostic;
pub mod gpu;
pub mod image;
pub mod input;
pub mod platform;
pub mod reactive;
pub mod scene;
pub mod text;
pub mod widget;

pub use apps::{CalculatorApp, CounterApp};
pub use diagnostic::{GuiError, GuiResult};
pub use gpu::{
    GpuCapabilities, GpuContext, GpuFrame, GpuSurface, HardwareTier, map_adapter_error,
    map_create_surface_error, map_device_error, map_surface_status,
};
pub use image::{ImageFit, ImageTexture};
pub use input::{ElementState, GuiEvent, Key, Modifiers, MouseButton};
pub use platform::{
    GuiApp, GuiEventLoop, GuiWindow, HeadlessEventSource, WindowConfig, map_os_error,
    map_window_event,
};
pub use reactive::{ReactiveBatch, Signal, SignalId};
pub use scene::{
    ClipShape, Color, Point, Rect, RoundedRect, SceneBuilder, SceneNode, SceneRenderer, Size,
};
pub use text::{FontContext, FontWeight, ShapedGlyph, ShapedTextLayout, TextAlign, TextWrap};
pub use widget::{
    Button, Card, CrossAxisAlignment, Flex, FlexDirection, Label, LayoutConstraints,
    MainAxisAlignment, UiNodeKey, Widget,
};
