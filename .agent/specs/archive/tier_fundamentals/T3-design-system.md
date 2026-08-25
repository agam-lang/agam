# Phase T3-design-system � Design System and Theming

**Status:** open
**Tier:** 1 (Developer Experience Excellence)
**Pillar:** 17 — Zero-Friction Visual Toolchain

## Vision

A world-class GUI language requires world-class visual tooling **baked in**. Hot-reloading, a visual UI architect, live preview, and design-to-code integration — all built into `agamc` from day one.

## Deliverables

### Hot-Reload
- [ ] `agamc dev --gui` watches source files and hot-reloads UI changes instantly
- [ ] Change a style property → running app updates in <100ms without restart
- [ ] Change a widget tree → running app diffs and patches without losing state
- [ ] Change business logic → hot-swap compiled function via JIT
- [ ] Font change (e.g., switching to Cascadia Code) → instant preview update

### Live Preview
- [ ] `agamc preview` renders UI in a lightweight preview window
- [ ] Side-by-side: code editor + live preview
- [ ] Component isolation: preview individual widgets with mock data
- [ ] Responsive preview: resize to simulate different screen sizes
- [ ] Theme preview: switch themes and see all components at once

### UI Architect Tool
- [ ] `agamc architect` — visual drag-and-drop UI builder
- [ ] Generates clean, idiomatic `@ui` Agam code (not binary layout files)
- [ ] Two-way sync: edit code → UI updates, edit visually → code updates
- [ ] Component palette with all standard widgets
- [ ] Constraint editor: set layout constraints visually
- [ ] Design token editor: colors, typography, spacing

### Inspection and Debugging
- [ ] Widget inspector: hover over any element to see its properties, state, constraints
- [ ] Layout debugger: visualize flexbox/grid with overlay lines
- [ ] Performance overlay: frame time, GPU usage, widget count, re-render highlights
- [ ] State inspector: view current reactive state tree and dependencies
- [ ] `@debug_layout` annotation renders layout guidelines on screen

### Design System Integration
- [ ] Import Figma design tokens via JSON
- [ ] Export component catalog as interactive documentation
- [ ] Style guide generation from theme definition
- [ ] Design-to-code: Figma plugin generates `@ui` code (stretch)

### VS Code Extension
- [ ] Inline UI preview in the editor
- [ ] Widget completion with live property documentation
- [ ] Color picker for theme colors
- [ ] Layout visualization on hover

## Responsible Crates

- `agam_ui` — hot-reload runtime, preview server, inspector
- `agam_driver` — `agamc dev --gui`, `agamc preview`, `agamc architect` commands
- `agam_lsp` — VS Code extension UI features
- `agam_jit` — hot-swap function replacement for live code changes

## Dependencies

- Phase T3-native-renderer–GUI4 (rendering, components, reactive syntax)
- Phase T1-lsp-production (LSP) — VS Code extension integration
- Phase T1-daemon-prewarm (incremental daemon) — warm state for instant rebuilds
