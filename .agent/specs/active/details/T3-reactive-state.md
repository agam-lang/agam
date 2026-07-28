# Phase T3-reactive-state � Reactive State Management

**Status:** open
**Tier:** 3 (Platform and Ecosystem Breadth)
**Pillar:** 15 — Modern Component Ecosystem

## Vision

Developers should never build basic UI elements from scratch. Agam ships with a **massive, modular standard component library** that natively understands modern UI paradigms — Bento Box grids, Neumorphic designs, Neobrutalist interfaces, and Glassmorphism — all achievable with a few lines of declarative code.

## Deliverables

### Core Widget Library (`agam_ui::widgets`)
- [ ] **Layout:** Row, Column, Stack, Grid, Flex, Spacer, Divider, ScrollView
- [ ] **Input:** Button, TextField, TextArea, Checkbox, Radio, Switch, Slider, DatePicker
- [ ] **Display:** Text, Image, Icon, Avatar, Badge, Chip, ProgressBar, Spinner
- [ ] **Navigation:** TabBar, Sidebar, Breadcrumb, Pagination, Drawer, AppBar
- [ ] **Feedback:** Toast, Snackbar, Dialog, Modal, Tooltip, Popover
- [ ] **Data:** Table, DataGrid, List, TreeView, VirtualScroll (100K+ items)
- [ ] **Charts:** LineChart, BarChart, PieChart, ScatterPlot, Heatmap (GPU-accelerated)
- [ ] **Media:** VideoPlayer, AudioPlayer, CodeEditor, MarkdownView, TerminalView

### Theme Engine
- [ ] `@theme("material")` — Google Material Design 3
- [ ] `@theme("fluent")` — Microsoft Fluent Design
- [ ] `@theme("cupertino")` — Apple Human Interface
- [ ] `@theme("bento")` — Bento Box grid layouts
- [ ] `@theme("neumorphic")` — Soft shadow neumorphism
- [ ] `@theme("neobrutalist")` — Bold, raw, high-contrast
- [ ] `@theme("glassmorphic")` — Frosted glass with blur
- [ ] Custom theme creation via `Theme { colors, typography, spacing, shadows }`
- [ ] Live theme switching without restart
- [ ] System theme auto-detection (light/dark, accent color)

### Typography System
- [ ] Built-in font stack: Inter, Roboto, JetBrains Mono, Cascadia Code
- [ ] Font loading from system, bundled, or URL
- [ ] Rich text with inline styling: bold, italic, code, links
- [ ] Responsive typography: scales with container size
- [ ] Subpixel rendering via GPU text engine (Phase T3-gpu-rendering)

### Icon System
- [ ] Bundled icon sets: Material Icons, Lucide, Phosphor
- [ ] SVG icon rendering (GPU-accelerated)
- [ ] Animated icons with state transitions
- [ ] Icon theming: color, size, weight respond to theme

### Styling Engine
- [ ] CSS-inspired property syntax: `padding: 16, border_radius: 8, shadow: soft`
- [ ] Style composition: `style = base_style.merge(hover_style)`
- [ ] Pseudo-states: `:hover`, `:active`, `:focus`, `:disabled`
- [ ] Media queries: `@media(width > 768) { ... }`
- [ ] CSS-in-Agam: no separate stylesheet files needed

### Accessibility by Default
- [ ] All standard widgets are accessible out of the box
- [ ] ARIA-equivalent semantic annotations
- [ ] Keyboard navigation: Tab order, focus management
- [ ] High contrast mode support
- [ ] Screen reader text for all interactive elements

## Responsible Crates

- `agam_ui` — widget library, theme engine, styling, accessibility
- `agam_std` — font loading, image decoding, SVG parsing

## Dependencies

- Phase T3-native-renderer (platform renderer) — windowing and rendering
- Phase T3-gpu-rendering (GPU engine) — hardware-accelerated painting
- Phase T0-object-model (object model) — widget trait hierarchy
