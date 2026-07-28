# Context Hygiene (Progressive Discovery)

This repository enforces **Progressive Discovery** to prevent token window bloat and context poisoning. AI agents must follow these strict guidelines when exploring the codebase.

## 1. No "Blind" Mass Readings

Do not use `cat`, `view_file` (without line limits), or blindly load entire `.rs` files that exceed 500 lines on your first attempt to understand a component.

## 2. Targeted Searching

Use precise tools to locate specific logic:

- `ast-grep` or `ripgrep` to find function signatures and trait implementations.
- If you find a relevant file, read only the specific line numbers that contain the relevant struct or function.

## 3. Spec Isolation

Do not load completed phases or archived specs into context unless strictly necessary for historical debugging. The *only* spec you should have fully loaded is the one currently in `.agent/specs/active/`. Use the `spec-archiver` skill to manage completed work.

## 4. MCP Server Discipline

- **GitHub MCP:** When asked about an issue, use the GitHub MCP server to fetch the specific issue data rather than asking the user to copy-paste web text.
- **Sequential Thinking MCP:** If a task involves designing a new compiler pass or refactoring the AST, invoke Sequential Thinking to plan your approach internally *before* writing any code.
