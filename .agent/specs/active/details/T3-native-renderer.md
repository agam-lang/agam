# Phase T3-native-renderer � Omni-Platform Native Renderer

**Status:** open
**Tier:** 3 (Platform and Ecosystem Breadth)
**Pillar:** 13 — Omni-Platform Renderer

## Vision

Compile a **single Agam codebase** into truly native interfaces across Windows, Linux, macOS, and Web. No Electron, no bloated wrappers — a lightweight unified rendering engine that produces pixel-perfect native UI on every platform while scaling down to web and mobile views without layout breakage.

## Architecture

```
Agam UI Code (declarative)
    │
    ▼
┌─────────────────────────────────┐
│  agam_ui Frontend (platform-    │
│  agnostic widget tree)          │
├─────────────────────────────────┤
│  Layout Engine (flexbox/grid)   │
│  Style Resolver (theme system)  │
│  Accessibility Tree Builder     │
└─────────┬───────────────────────┘
          │
    ┌─────┼──────┬──────────┬────────────┐
    ▼     ▼      ▼          ▼            ▼
  Win32  Cocoa  GTK/      WASM/       Android
  DComp  Metal  Wayland   Canvas/     SurfaceView
  D3D12         Vulkan    WebGPU      Vulkan
```

## Deliverables

### Platform-Agnostic Widget Tree
- [ ] `agam_ui::Widget` trait with layout, paint, and event handling
- [ ] Virtual DOM-style diffing: only re-render changed subtrees
- [ ] Retained-mode scene graph with incremental updates
- [ ] Widget identity tracking for animation continuity

### Native Backend Renderers
- [ ] **Windows:** DirectComposition + Direct3D 12 for GPU-accelerated compositing
- [ ] **macOS:** Core Animation + Metal for native feel
- [ ] **Linux:** Wayland/X11 + Vulkan for modern desktop
- [ ] **Web:** Canvas 2D / WebGPU via WASM (extends Phase T3-wasm-backend)
- [ ] **Android:** SurfaceView + Vulkan (extends existing Android target)

### Native Look-and-Feel
- [ ] Platform-native window chrome (title bars, buttons, menus)
- [ ] System theme detection (light/dark mode, accent colors)
- [ ] Platform-specific text rendering (DirectWrite, CoreText, FreeType/HarfBuzz)
- [ ] Native file dialogs, notifications, system tray
- [ ] Accessibility: screen reader support on every platform (MSAA/UIA, NSAccessibility, ATK)

### Responsive Layout
- [ ] Flexbox and CSS Grid-inspired layout engine
- [ ] Breakpoint system: desktop → tablet → mobile → widget
- [ ] Layout constraints: min/max size, aspect ratio, alignment
- [ ] `@responsive` annotation for adaptive layouts

### Single Codebase Compilation
- [ ] `agamc build --target windows-gui` → native `.exe` with embedded resources
- [ ] `agamc build --target macos-gui` → `.app` bundle
- [ ] `agamc build --target linux-gui` → native binary with Wayland/X11
- [ ] `agamc build --target web-gui` → WASM + HTML + JS shell
- [ ] `agamc build --target android-gui` → APK with native rendering

## Responsible Crates

- `agam_ui` — widget tree, layout engine, style resolver, event system
- `agam_codegen` — platform-specific renderer backends
- `agam_runtime` — event loop integration, window management
- `agam_driver` — GUI build targets

## Dependencies

- Phase T0-type-system/F3 (type system, object model) — widget traits need generics
- Phase T3-wasm-backend (WASM) — web rendering target
- Phase T3-gpu-rendering (GPU rendering) — hardware-accelerated paint
