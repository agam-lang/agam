---
name: graphify
description: >
  Turn any folder of code, docs, papers, images into a queryable knowledge graph.
  Outputs interactive HTML, GraphRAG-ready JSON, and a plain-language audit report.
  Requires: pip install graphifyy && graphify install
  Trigger: /graphify [path] [--options]
---

Turn any folder of files into navigable knowledge graph with community detection, honest audit trail, and three outputs: interactive HTML, queryable JSON, and plain-language GRAPH_REPORT.md.

## Prerequisites

```bash
pip install graphifyy
graphify install                    # Claude Code
graphify install --platform windows # Windows-specific
graphify antigravity install        # Google Antigravity
graphify install --platform gemini  # Gemini CLI
graphify install --platform codex   # Codex
```

## Usage
```
/graphify                                  # full pipeline on current directory
/graphify <path>                           # full pipeline on specific path
/graphify <path> --mode deep               # thorough extraction, richer INFERRED edges
/graphify <path> --update                  # incremental - re-extract only new/changed files
/graphify <path> --directed                # build directed graph (preserves edge direction)
/graphify <path> --cluster-only            # rerun clustering on existing graph
/graphify <path> --no-viz                  # skip visualization, just report + JSON
/graphify <path> --obsidian                # generate Obsidian vault (one note per node)
/graphify <path> --obsidian --obsidian-dir ~/vaults/my-project  # custom vault path
/graphify <path> --svg                     # also export graph.svg
/graphify <path> --graphml                 # export graph.graphml (Gephi, yEd)
/graphify <path> --neo4j                   # generate cypher.txt for Neo4j
/graphify <path> --wiki                    # build agent-crawlable wiki
/graphify query "<question>"               # BFS traversal - broad context
/graphify query "<question>" --dfs         # DFS - trace a specific path
/graphify path "NodeA" "NodeB"             # shortest path between two concepts
/graphify explain "ConceptName"            # plain-language explanation of a node
```

## Outputs
```
graphify-out/
├── graph.html           interactive graph - click nodes, search, filter by community
├── GRAPH_REPORT.md      god nodes, surprising connections, suggested questions
├── graph.json           persistent graph - query weeks later without re-reading
└── cache/               SHA256 cache - re-runs only process changed files
```

## How It Works
1. **AST pass** — deterministic structure extraction from code (no LLM needed)
2. **Semantic pass** — parallel subagents extract concepts from docs/papers/images
3. **Merge** — AST + semantic results combined into NetworkX graph
4. **Cluster** — Leiden community detection by edge density
5. **Report** — god nodes, surprising connections, suggested questions

Every relationship tagged: EXTRACTED (found in source), INFERRED (reasonable inference), or AMBIGUOUS (flagged for review).

## Integration with Agam

After building graph, run `graphify antigravity install` to:
- Add awareness to agent context files
- Install hooks that check graph before grepping raw files
- Enable architecture-first navigation

## Key Value
- **71.5x fewer tokens** per query vs reading raw files
- **Persistent across sessions** — graph survives session restarts
- **Honest** — always shows what was found vs guessed
- **25 languages** via tree-sitter AST (including Rust)

For full pipeline details, see: https://github.com/safishamsi/graphify
