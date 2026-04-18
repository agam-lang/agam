---
name: graphify
description: >
  Turn any folder of code, docs, papers, images into a queryable knowledge graph.
  Outputs interactive HTML, GraphRAG-ready JSON, and a plain-language audit report.
  Requires: pip install graphifyy && graphify install
  Trigger: /graphify [path] [--options]
---

Turn codebase into navigable knowledge graph with community detection and honest audit trail.

## Usage
```
/graphify                           # full pipeline on current directory
/graphify <path>                    # specific path
/graphify <path> --update           # incremental — only new/changed files
/graphify <path> --obsidian         # generate Obsidian vault
/graphify <path> --obsidian --obsidian-dir ~/vaults/agam  # custom vault path
/graphify query "<question>"        # query the graph
/graphify path "NodeA" "NodeB"      # shortest path between concepts
```

## Outputs
```
graphify-out/
├── graph.html           interactive graph
├── GRAPH_REPORT.md      god nodes, connections, questions
├── graph.json           persistent queryable graph
└── cache/               SHA256 cache for incremental runs
```

## How It Works
1. AST pass — struct extraction from Rust/code (no LLM)
2. Semantic pass — parallel subagents on docs/papers/images
3. Merge → Leiden clustering → report generation

Every edge tagged: EXTRACTED, INFERRED, or AMBIGUOUS.

## Value for Agam Compiler
- 26 crates, ~8900-line driver — graph shows god nodes + community structure
- Navigate architecture without grepping every file
- 71.5x fewer tokens per architecture query

For full docs: https://github.com/safishamsi/graphify
