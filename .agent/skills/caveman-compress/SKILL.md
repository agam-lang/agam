---
name: caveman-compress
description: >
  Compress natural language memory files (CLAUDE.md, AGENTS.md, todos, preferences) into caveman format
  to save input tokens. Preserves all technical substance, code, URLs, and structure.
  Compressed version overwrites the original file. Human-readable backup saved as FILE.original.md.
  Trigger: /caveman:compress <filepath> or "compress memory file"
---

## Purpose
Compress natural language files into caveman-speak to reduce input tokens. Backup saved as `<filename>.original.md`.

## Trigger
`/caveman:compress <filepath>` or when user asks to compress a memory file.

### Remove
- Articles, filler, pleasantries, hedging, redundant phrasing, connective fluff

### Preserve EXACTLY
- Code blocks, inline code, URLs, file paths, commands, technical terms, proper nouns, dates, versions

### Preserve Structure
- Markdown headings, bullet hierarchy, numbered lists, tables, frontmatter

### Compress
- Short synonyms, fragments OK, drop "you should"/"make sure to", merge redundant bullets

CRITICAL: Anything inside ``` ... ``` copied EXACTLY. Inline code unchanged.

## Boundaries
- ONLY compress: .md, .txt, extensionless
- NEVER modify: .py, .js, .ts, .json, .yaml, .toml, .env, .lock, .rs, .sh
- Original backed up as FILE.original.md before overwriting
