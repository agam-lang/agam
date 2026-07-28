# Phase T3-generative-ui � Generative UI and Agent-Driven Interfaces

**Status:** open
**Tier:** 3 (Platform and Ecosystem)
**Pillar:** 16 (Declarative & Reactive Syntax)

## Vision

Enable Agam's UI framework to support **Generative UI** — where AI agents dynamically compose and render interfaces at runtime using Agam's declarative `@ui` DSL and a type-safe component catalog.

## Motivation

In 2026, Generative UI (GenUI) is the dominant pattern for AI-powered applications:
- **A2UI (Google)**: Agent-to-User Interface protocol for agents to "speak" UI
- **CopilotKit**: Framework for AI-driven React component composition
- **AG-UI Protocol**: Streaming state sync between agents and frontends
- **MCP**: Model Context Protocol connecting agents to UI resources

Agam's planned declarative UI syntax (Phase T3-declarative-ui) and MCP server (Phase T1-compiler-agent-tool) provide the ideal foundation for GenUI — the compiler knows the component catalog, and the agent can query it.

## Deliverables

### Component Catalog API
- [ ] `@component` annotation: register UI components in a queryable catalog
- [ ] Component schema export: JSON schema describing each component's props/slots
- [ ] `agam_std::ui::catalog()` — runtime query of available components
- [ ] Component metadata: name, description, category, preview thumbnail

### Agent-Driven Composition
- [ ] `ui::render_from_schema(json)` — render UI from agent-generated JSON schema
- [ ] Type-safe validation of agent-generated UI descriptions
- [ ] Sandboxed rendering: agent cannot access DOM/state outside its component scope
- [ ] Streaming UI updates: agent can progressively build/modify the interface

### Protocols
- [ ] A2UI protocol support for agent-to-UI communication
- [ ] MCP resource: expose UI catalog as MCP resource (coordinates with AGENT1)
- [ ] WebSocket-based state synchronization for real-time agent-UI interaction

### Design Surface
- [ ] `@ui` DSL supports dynamic component insertion points (`<slot agent="true">`)
- [ ] Theme-aware: agent-composed UI respects the application's design system
- [ ] Accessibility: generated UI must pass WCAG 2.1 AA automatically

## Design References

- **A2UI (Google/Android)**: Declarative agent-to-UI bridge. Secure, type-safe.
- **CopilotKit**: React framework for AI-driven UI composition.
- **Agam advantage**: Compiler knows the component catalog at compile time → can validate agent-generated UI schemas statically.

## Responsible Crates

- `agam_ui` — component catalog, schema export, rendering pipeline
- `agam_std` — `ui::catalog()` API
- `agam_driver` — MCP UI resource exposure

## Dependencies

- Phase T3-declarative-ui (Declarative UI Syntax) — `@ui` DSL and component model
- Phase T1-compiler-agent-tool (Compiler-as-Agent-Tool) — MCP server for agent integration
- Phase T3-native-renderer (Omni-Platform Renderer) — rendering backend
