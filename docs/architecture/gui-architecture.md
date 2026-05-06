# Agam GUI Architecture — World-Class Visual Engine

> This document defines the target architecture for Agam's omni-platform GUI subsystem.
> It covers the rendering pipeline, reactive runtime, component ecosystem, and visual toolchain.

---

## 1. Design Goals

| Goal | Target | Comparison |
|------|--------|------------|
| Frame rate | 120 FPS sustained | Better than Electron (30), Flutter (60) |
| Memory usage | <50MB for a standard desktop app | 10x less than Electron |
| Startup time | <200ms to first paint | Competitive with native Qt/GTK |
| Cross-platform | Single codebase → Win/Mac/Linux/Web/Android | Like Flutter but native rendering |
| Component count | 100+ production widgets at launch | Like Material UI, not a bare framework |
| Hot-reload | <100ms for style changes, <500ms for logic | Like Flutter's hot-reload |

---

## 2. Architecture Overview

```
                    ┌──────────────────────────────┐
                    │  Developer Code               │
                    │                                │
                    │  @ui fn app() -> View:         │
                    │    Column:                     │
                    │      Text("Hello")             │
                    │      Button("Click me")        │
                    └──────────┬───────────────────┘
                               │ Parse @ui DSL
                               ▼
                    ┌──────────────────────────────┐
                    │  Reactive Runtime              │
                    │                                │
                    │  Observable State Tracking      │
                    │  Dependency Graph               │
                    │  Change Detection               │
                    │  Batched Re-render Scheduling   │
                    └──────────┬───────────────────┘
                               │ Widget Tree Diff
                               ▼
                    ┌──────────────────────────────┐
                    │  Layout Engine                  │
                    │                                │
                    │  Flexbox + Grid Solver          │
                    │  Constraint Propagation         │
                    │  Text Shaping (HarfBuzz)        │
                    │  Responsive Breakpoints         │
                    └──────────┬───────────────────┘
                               │ Display List
                               ▼
                    ┌──────────────────────────────┐
                    │  GPU Render Pipeline            │
                    │                                │
                    │  Scene Graph → Draw Commands    │
                    │  Batch & Instance               │
                    │  Glyph Atlas (SDF)              │
                    │  Anti-Aliased Paths             │
                    │  Animation Interpolation        │
                    └──────────┬───────────────────┘
                               │
              ┌────────┬───────┼───────┬────────────┐
              ▼        ▼       ▼       ▼            ▼
          ┌───────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌────────┐
          │ D3D12 │ │Metal │ │Vulkan│ │WebGPU│ │Software│
          │ Win32 │ │Cocoa │ │ GTK  │ │Canvas│ │  CPU   │
          └───────┘ └──────┘ └──────┘ └──────┘ └────────┘
          Windows   macOS    Linux     Web      Headless
```

---

## 3. Reactive Runtime Design

### State System

```rust
// Observable state — changes automatically trigger re-render
@observable
struct AppState {
    count: i32 = 0
    items: Vec<String> = []
    theme: ThemeMode = .dark
}

// The runtime builds a dependency graph:
//   count → [CounterLabel, TotalDisplay]
//   items → [ItemList, ItemCount]
//   theme → [entire tree]
//
// When count changes, ONLY CounterLabel and TotalDisplay re-render.
```

### Reactive Execution Model

```
State Change (e.g., count += 1)
    │
    ▼
Dependency Graph Lookup
    │ → Which widgets read `count`?
    ▼
Mark Dirty Widgets
    │
    ▼
Batch (coalesce multiple changes in same frame)
    │
    ▼
Re-execute dirty @ui functions
    │
    ▼
Diff old widget tree vs new widget tree
    │
    ▼
Patch display list (minimal GPU updates)
    │
    ▼
Submit to GPU (next vsync)
```

**Key invariant:** No widget re-executes unless its dependencies changed. This is O(changed) not O(total).

---

## 4. GPU Render Pipeline

### Text Rendering (SDF-Based)

```
Font File → Glyph Outlines → SDF Generation (offline) → GPU Texture Atlas
                                                              │
    Text Layout (HarfBuzz) → Glyph Positions ─────────────────┤
                                                              ▼
                                                    GPU Fragment Shader
                                                    (SDF sampling, AA)
                                                              │
                                                              ▼
                                                    Crisp text at any zoom
```

- Signed Distance Field (SDF) text renders at any scale without re-rasterization
- GPU glyph atlas caches frequently used characters
- Sub-pixel positioning for LCD-quality rendering

### 2D Vector Graphics

```
Widget Paint Call → Path Commands → Tessellation → GPU Triangle Mesh
                                                        │
                                        Anti-aliasing via MSAA or
                                        analytical coverage shader
```

- Rounded rectangles, shadows, and gradients are GPU shaders, not CPU raster
- Blur effects (glassmorphism) use GPU compute passes
- Drop shadows computed in a single GPU pass with configurable softness

### Compositing

```
Layer 0: Background (solid or gradient)
Layer 1: Content widgets (batched draw calls)
Layer 2: Overlay effects (blur, shadow, glow)
Layer 3: Foreground (tooltips, dropdowns, modals)
    │
    ▼
GPU Compositor: blend all layers → final framebuffer → display
```

---

## 5. Platform Abstraction Layer

### Window Management

```rust
trait PlatformWindow {
    fn create(config: WindowConfig) -> Self;
    fn set_title(&mut self, title: &str);
    fn resize(&mut self, size: Size);
    fn request_redraw(&self);
    fn poll_events(&self) -> Vec<Event>;
    fn gpu_surface(&self) -> &dyn GpuSurface;
}

// Platform implementations:
// WindowsWindow  → Win32 + DXGI
// CocoaWindow    → NSWindow + CAMetalLayer
// WaylandWindow  → wl_surface + VkSurfaceKHR
// WebWindow      → <canvas> + WebGPU
// AndroidWindow  → ANativeWindow + VkSurfaceKHR
```

### Native Integration Points

| Feature | Windows | macOS | Linux | Web | Android |
|---------|---------|-------|-------|-----|---------|
| Window chrome | Win32 DWM | NSWindow | CSD/SSD | CSS | Activity |
| Text rendering | DirectWrite | CoreText | FreeType | Canvas | Skia |
| GPU API | D3D12 | Metal | Vulkan | WebGPU | Vulkan |
| File dialog | IFileDialog | NSOpenPanel | portal | \<input\> | Intent |
| Clipboard | Win32 | NSPasteboard | wl_data | Clipboard API | ClipboardManager |
| Notifications | ToastNotification | UNNotification | DBus | Notification API | NotificationManager |
| Accessibility | UIA/MSAA | NSAccessibility | ATK/AT-SPI | ARIA | AccessibilityService |
| System theme | Registry | NSApp.effectiveAppearance | portal | media query | Configuration |

---

## 6. Component Architecture

### Widget Trait

```rust
trait Widget {
    /// Compute minimum, preferred, and maximum size
    fn layout(&self, constraints: Constraints) -> Size;

    /// Generate paint commands for the GPU render pipeline
    fn paint(&self, canvas: &mut Canvas);

    /// Handle input events, return whether consumed
    fn event(&mut self, event: &Event) -> EventResult;

    /// Return child widgets for tree traversal
    fn children(&self) -> &[WidgetRef];

    /// Accessibility properties
    fn accessibility(&self) -> AccessibilityNode;
}
```

### Theme System

```rust
struct Theme {
    colors: ColorPalette,      // primary, secondary, surface, error, ...
    typography: TypographyScale, // display, headline, title, body, label
    spacing: SpacingScale,     // xs=4, sm=8, md=16, lg=24, xl=32
    shape: ShapeScale,         // border radii per component category
    elevation: ElevationScale, // shadow definitions per elevation level
    motion: MotionScale,       // animation duration and easing per category
}

// Built-in themes:
// Theme::material()      → Material Design 3
// Theme::fluent()        → Microsoft Fluent
// Theme::cupertino()     → Apple HIG
// Theme::bento()         → Bento Box grid
// Theme::neumorphic()    → Soft shadows
// Theme::neobrutalist()  → Bold, raw
// Theme::glassmorphic()  → Frosted glass
```

---

## 7. Hot-Reload Architecture

```
Source File Change Detected (fs watcher)
    │
    ▼
Incremental Re-parse (daemon warm state)
    │
    ├─ Style-only change? → Patch theme/style properties in-place
    │                        (no widget tree rebuild, <100ms)
    │
    ├─ Widget tree change? → Diff old vs new @ui function output
    │                        Patch widget tree, preserve state (<200ms)
    │
    └─ Logic change? → JIT-recompile changed function
                       Hot-swap function pointer in running app
                       Re-trigger reactive dependencies (<500ms)
```

**State preservation:** App state (`@observable`, `@state`) is maintained across hot-reloads. Only the UI description and business logic are swapped.

---

## 8. New Crates

```
crates/
├── runtime/
│   └── agam_ui/                [EXPAND from stub]
│       ├── src/
│       │   ├── lib.rs
│       │   ├── widget/         Widget trait, base widgets, tree
│       │   ├── layout/         Flexbox, grid solver, constraints
│       │   ├── render/         GPU pipeline, scene graph, display list
│       │   ├── reactive/       Observable state, dependency graph, scheduler
│       │   ├── theme/          Theme engine, built-in themes
│       │   ├── animation/      Spring physics, keyframes, easing
│       │   ├── accessibility/  A11y tree, screen reader integration
│       │   ├── text/           Text shaping, SDF atlas, rich text
│       │   ├── platform/       Platform abstraction layer
│       │   │   ├── windows.rs
│       │   │   ├── macos.rs
│       │   │   ├── linux.rs
│       │   │   ├── web.rs
│       │   │   └── android.rs
│       │   └── tools/          Hot-reload, preview, inspector, architect
│       └── widgets/            Standard widget library (100+)
│           ├── button.rs
│           ├── text_field.rs
│           ├── data_grid.rs
│           ├── chart.rs
│           └── ...
```

---

## 9. Performance Budget

| Operation | Budget | Strategy |
|-----------|--------|----------|
| Frame render | <8.3ms (120 FPS) | GPU-only rendering, batched draws |
| State change → screen update | <16.6ms (within same frame) | Reactive diff, minimal repaint |
| Hot-reload (style) | <100ms | In-place property patch |
| Hot-reload (widget tree) | <200ms | Incremental diff + JIT |
| Hot-reload (logic) | <500ms | Function-level JIT swap |
| App startup → first paint | <200ms | Pre-compiled widget tree, GPU warmup |
| Scroll 10K items | 60 FPS | Virtual scrolling, GPU instancing |
| Chart with 100K points | 60 FPS | GPU-computed, level-of-detail |
| Memory per widget | <1KB | Arena allocation, shared styles |
| Idle CPU usage | <1% | Event-driven, no polling |
