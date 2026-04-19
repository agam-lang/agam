import os

BASE_DIR = r"c:\Users\ksvik\IdeaProjects\Agam-Lang"

# ============================================================
# REPO DEFINITIONS: each repo gets tailored agent context,
# rich READMEs, proper rules, phases, skills, and policies
# ============================================================

REPOS = {
    "agamlab": {
        "desc": "AGAMLAB — MATLAB-like Interactive Scientific Computing Platform",
        "mission": "Provide a MATLAB-class interactive scientific computing environment built natively on top of the Agam language and toolchain.",
        "tech": "Rust workspace with Agam-native numerical libraries",
        "areas": ["Matrix/Linear Algebra", "Signal Processing", "Statistics", "Plotting/Visualization", "Interactive REPL", "Notebook Interface"],
        "crates": ["agamlab_core", "agamlab_repl", "agamlab_notebook", "agamlab_plot", "agamlab_matrix", "agamlab_signal", "agamlab_stats", "agamlab_io"],
        "phases": [
            ("Phase 1", "Core Engine", "Build the computation engine on top of agam_std numerical primitives", "planned"),
            ("Phase 2", "Interactive REPL", "Implement MATLAB-like command window with history and tab completion", "planned"),
            ("Phase 3", "Matrix Operations", "First-class matrix types, decompositions, solvers", "planned"),
            ("Phase 4", "Plotting", "Built-in 2D/3D visualization pipeline", "planned"),
            ("Phase 5", "Notebook Interface", "Jupyter-style notebook but native Agam", "planned"),
            ("Phase 6", "Signal Processing", "FFT, filters, spectral analysis toolbox", "planned"),
            ("Phase 7", "Statistics", "Distributions, hypothesis testing, regression", "planned"),
        ],
        "rules": {
            "numerical-accuracy.md": "# Numerical Accuracy Rules\n\n- All matrix operations must preserve numerical stability.\n- Use compensated summation (Kahan) for reductions over large arrays.\n- Document precision guarantees for every public API.\n- Test edge cases: NaN, Inf, denormals, near-zero determinants.\n",
            "api-design.md": "# API Design Rules\n\n- AGAMLAB APIs should feel familiar to MATLAB and NumPy users.\n- Function names should be concise and domain-standard (e.g., `fft`, `svd`, `linspace`).\n- All public functions must include docstrings with examples.\n- Prefer value semantics for small matrices, reference semantics for large data.\n",
        },
        "skills": {
            "numerical-validation": {
                "SKILL.md": '---\nname: numerical-validation\ndescription: Use when implementing or modifying numerical algorithms to ensure correctness and precision.\n---\n\n# Numerical Validation\n\nUse this skill when working on matrix operations, signal processing, or statistical functions.\n\n## Workflow\n\n1. Identify the algorithm and its known numerical properties.\n2. Compare results against reference implementations (MATLAB, NumPy, SciPy).\n3. Test with pathological inputs: singular matrices, near-zero values, very large arrays.\n4. Document precision bounds in the function docstring.\n5. Add regression tests for edge cases discovered during validation.\n\n## Success Criteria\n\n- Results match reference implementations within documented precision bounds\n- Edge cases are explicitly handled and tested\n- No silent precision loss in reduction operations\n',
            },
        },
    },
    "std": {
        "desc": "Agam Standard Library",
        "mission": "Provide the foundational standard library for the Agam language, covering collections, I/O, math, concurrency, networking, and tensor operations.",
        "tech": "Agam source modules + Rust runtime backing",
        "areas": ["Collections", "I/O", "Math/Numerical", "Concurrency", "Networking", "String Processing", "Tensor Operations"],
        "crates": [],
        "phases": [
            ("Phase 1", "Core Types", "Implement fundamental types: Array, Map, Set, String, Option, Result", "planned"),
            ("Phase 2", "I/O", "File I/O, stdin/stdout, path manipulation", "planned"),
            ("Phase 3", "Math", "Mathematical functions, constants, random number generation", "planned"),
            ("Phase 4", "Collections", "Advanced collection types and iterators", "planned"),
            ("Phase 5", "Concurrency", "Async primitives, channels, thread pools", "planned"),
            ("Phase 6", "Networking", "TCP/UDP sockets, HTTP client", "planned"),
            ("Phase 7", "Tensor", "N-dimensional array and tensor operations", "planned"),
        ],
        "rules": {
            "api-stability.md": "# API Stability Rules\n\n- All public APIs in std are part of the language contract.\n- Breaking changes require an RFC and a deprecation cycle.\n- Every public function must have documentation and at least one test.\n- Prefer zero-cost abstractions that compile to optimal native code.\n",
            "portability.md": "# Portability Rules\n\n- Standard library modules must work on all supported Agam targets (Windows, Linux, Android).\n- Platform-specific code must be isolated behind target-conditional compilation.\n- Test on at least two platforms before marking a module as stable.\n",
        },
        "skills": {},
    },
    "registry-index": {
        "desc": "Agam Package Registry Index",
        "mission": "Provide the central package discovery and metadata index for the Agam package ecosystem, following the design in the core repo's package-ecosystem.md policy.",
        "tech": "JSON metadata index + Agam CLI integration",
        "areas": ["Package Metadata", "Version Resolution", "Publication Protocol", "Search & Discovery"],
        "crates": [],
        "phases": [
            ("Phase 1", "Index Schema", "Define the registry index format, package metadata schema, and directory structure", "planned"),
            ("Phase 2", "Publication Protocol", "Implement the publish/validate/index pipeline for agamc publish", "planned"),
            ("Phase 3", "Search & Discovery", "Package search, categorization, and featured packages", "planned"),
        ],
        "rules": {
            "immutability.md": "# Registry Immutability Rules\n\n- Published package versions are immutable. Never allow overwrites.\n- Every release must include cryptographic checksums and provenance data.\n- Yanked packages remain in the index with a yank marker, they are not deleted.\n",
        },
        "skills": {},
    },
    "sdk-packs": {
        "desc": "Agam SDK & Toolchain Bundles",
        "mission": "Provide pre-built SDK bundles, LLVM toolchain packs, target packs, and sysroot distributions for Agam development.",
        "tech": "SDK manifest JSON + platform-specific binary bundles",
        "areas": ["LLVM Bundles", "Target Packs", "Sysroot Distribution", "SDK Manifests"],
        "crates": [],
        "phases": [
            ("Phase 1", "SDK Manifest Specification", "Define sdk-manifest.json schema and bundle layout", "planned"),
            ("Phase 2", "Windows SDK Pack", "Pre-built LLVM + Windows sysroot bundle", "planned"),
            ("Phase 3", "Linux SDK Pack", "Pre-built LLVM + Linux sysroot bundle", "planned"),
            ("Phase 4", "Android Target Pack", "NDK sysroot + ARM64 target pack", "planned"),
        ],
        "rules": {
            "bundle-verification.md": "# Bundle Verification Rules\n\n- Each SDK bundle must pass agamc doctor validation on the target platform.\n- Bundle checksums must be reproducible across builds.\n- SDK manifests must declare exact LLVM version, target triple, and sysroot layout.\n",
        },
        "skills": {},
    },
    "rfcs": {
        "desc": "Agam Language RFCs",
        "mission": "Provide a structured process for proposing and deciding on language design changes, new features, and breaking modifications to the Agam language and ecosystem.",
        "tech": "Markdown RFC documents",
        "areas": ["Language Design", "Compiler Features", "Standard Library", "Tooling", "Ecosystem"],
        "crates": [],
        "phases": [
            ("Phase 1", "RFC Process", "Establish the RFC template, submission process, and review workflow", "active"),
            ("Phase 2", "Backfill", "Document existing language decisions as informational RFCs", "planned"),
        ],
        "rules": {
            "rfc-process.md": "# RFC Process Rules\n\n- Every language-visible change should go through an RFC.\n- RFCs must include motivation, detailed design, drawbacks, and alternatives.\n- Accepted RFCs are merged into the main branch. Rejected RFCs are closed with rationale.\n- Implementation may only begin after an RFC is accepted.\n",
        },
        "skills": {},
    },
    "governance": {
        "desc": "Agam Organization Governance",
        "mission": "Define and maintain the organizational policies, team structure, decision-making processes, and community standards for the agam-lang organization.",
        "tech": "Markdown policy documents",
        "areas": ["Team Structure", "Decision Making", "Code of Conduct", "Release Process", "Security Policy"],
        "crates": [],
        "phases": [
            ("Phase 1", "Foundation", "Establish core governance documents, team roles, and decision processes", "active"),
            ("Phase 2", "Community Growth", "Contributor mentoring program, community guidelines expansion", "planned"),
        ],
        "rules": {
            "transparency.md": "# Transparency Rules\n\n- All governance decisions must be documented with rationale.\n- Team membership changes are announced publicly.\n- Financial decisions (sponsorship allocation) are tracked in the open.\n",
        },
        "skills": {},
    },
    "agam-vscode": {
        "desc": "Agam VS Code Extension",
        "mission": "Provide first-class VS Code support for Agam development including syntax highlighting, LSP integration, snippets, debugging, and code actions.",
        "tech": "TypeScript + VS Code Extension API",
        "areas": ["Syntax Highlighting", "LSP Client", "Snippets", "Debugging", "Code Actions", "Themes"],
        "crates": [],
        "phases": [
            ("Phase 1", "Syntax Highlighting", "TextMate grammar for .agam files covering all three language modes", "planned"),
            ("Phase 2", "LSP Integration", "Connect to agam_lsp for diagnostics, completions, and hover", "planned"),
            ("Phase 3", "Snippets", "Code snippets for common Agam patterns", "planned"),
            ("Phase 4", "Debugging", "DAP integration for JIT and native debugging", "planned"),
            ("Phase 5", "Code Actions", "Quick fixes, refactoring support, format-on-save", "planned"),
        ],
        "rules": {
            "extension-quality.md": "# Extension Quality Rules\n\n- The extension must not slow down VS Code startup by more than 100ms.\n- All TextMate grammar rules must be tested against real .agam files from the core repo.\n- LSP features must gracefully degrade when the language server is unavailable.\n",
        },
        "skills": {},
    },
    "agam-intellij": {
        "desc": "Agam IntelliJ Plugin",
        "mission": "Provide IntelliJ/IDEA integration for Agam development with syntax highlighting, project support, and eventual LSP connection.",
        "tech": "Kotlin/Java + IntelliJ Platform SDK",
        "areas": ["Syntax Highlighting", "Project Support", "LSP Integration"],
        "crates": [],
        "phases": [
            ("Phase 1", "Syntax Highlighting", "Lexer-based syntax highlighting for .agam files", "planned"),
            ("Phase 2", "Project Support", "Agam project type, run configurations", "planned"),
            ("Phase 3", "LSP Integration", "Connect to agam_lsp for full IDE features", "planned"),
        ],
        "rules": {},
        "skills": {},
    },
    "playground": {
        "desc": "Agam Web Playground",
        "mission": "Provide a web-based environment for trying Agam code online without any local installation, powered by WASM compilation of the Agam JIT backend.",
        "tech": "Web frontend + WASM-compiled Agam JIT",
        "areas": ["Code Editor", "WASM Compilation", "Output Display", "Example Gallery"],
        "crates": [],
        "phases": [
            ("Phase 1", "Editor UI", "Monaco-based code editor with Agam syntax highlighting", "planned"),
            ("Phase 2", "WASM Backend", "Compile agam_jit to WASM for browser execution", "planned"),
            ("Phase 3", "Example Gallery", "Pre-loaded examples showcasing Agam features", "planned"),
        ],
        "rules": {},
        "skills": {},
    },
    "examples": {
        "desc": "Agam Examples & Tutorials",
        "mission": "Provide curated, well-documented example projects and tutorials that showcase Agam's capabilities across systems, AI, and scientific computing domains.",
        "tech": "Agam source files + documentation",
        "areas": ["Getting Started", "Systems Programming", "AI/ML", "Scientific Computing", "Web & Networking"],
        "crates": [],
        "phases": [
            ("Phase 1", "Getting Started", "Hello world, basic syntax tour, project setup", "planned"),
            ("Phase 2", "Domain Examples", "Systems, AI/ML, scientific computing showcases", "planned"),
            ("Phase 3", "Tutorial Series", "Step-by-step guided tutorials with explanations", "planned"),
        ],
        "rules": {
            "example-quality.md": "# Example Quality Rules\n\n- Every example must compile and run successfully with the latest agamc.\n- Examples must include comments explaining non-obvious code.\n- Each example directory must have a README explaining what it demonstrates.\n- Examples should showcase Agam's strengths, not just basic syntax.\n",
        },
        "skills": {},
    },
    "benchmarks": {
        "desc": "Agam Cross-Language Benchmark Suite",
        "mission": "Provide a standalone, reproducible cross-language benchmark suite for comparing Agam's performance against C, C++, Rust, Go, Python, and other languages.",
        "tech": "Agam + comparison language sources + Python harness",
        "areas": ["Algorithm Benchmarks", "Numerical Benchmarks", "Memory Benchmarks", "Compile-Time Benchmarks"],
        "crates": [],
        "phases": [
            ("Phase 1", "Algorithm Suite", "Fibonacci, sorting, graph algorithms across all target languages", "planned"),
            ("Phase 2", "Numerical Suite", "Matrix multiply, FFT, linear algebra benchmarks", "planned"),
            ("Phase 3", "Real-World Suite", "HTTP server, JSON parsing, file processing benchmarks", "planned"),
        ],
        "rules": {
            "methodology.md": "# Benchmark Methodology Rules\n\n- All benchmarks must run on the same host during the same session.\n- Warmup runs must precede measurement runs.\n- Report median, not mean, to reduce outlier impact.\n- Document the hardware, OS, and compiler versions used.\n- Never cherry-pick results. Report all targets measured.\n",
        },
        "skills": {
            "benchmark-guard": {
                "SKILL.md": '---\nname: benchmark-guard\ndescription: Use when changes claim performance wins or affect benchmark infrastructure.\n---\n\n# Benchmark Guard\n\nUse this skill when modifying benchmark suites or interpreting results.\n\n## Workflow\n\n1. Verify all comparison targets use equivalent optimization levels.\n2. Run warmup before measurement.\n3. Use median timing over at least 5 runs.\n4. Document any anomalies or outliers.\n5. Reject results from shared CI runners for cross-language performance claims.\n\n## Success Criteria\n\n- Results are reproducible on the same hardware\n- Methodology is documented alongside results\n- No cherry-picked or misleading comparisons\n',
            },
        },
    },
    "agam-lang.github.io": {
        "desc": "Agam Documentation Website",
        "mission": "Provide the official documentation website for the Agam language including language guide, API reference, tutorials, and blog.",
        "tech": "Static site (mdBook or Docusaurus)",
        "areas": ["Language Guide", "API Reference", "Tutorials", "Blog", "Getting Started"],
        "crates": [],
        "phases": [
            ("Phase 1", "Site Infrastructure", "Set up static site generator, CI deployment to GitHub Pages", "planned"),
            ("Phase 2", "Language Guide", "Core language documentation covering syntax, types, and semantics", "planned"),
            ("Phase 3", "API Reference", "Auto-generated standard library documentation", "planned"),
            ("Phase 4", "Tutorials", "Step-by-step tutorials for common use cases", "planned"),
            ("Phase 5", "Blog", "Development blog for release announcements and technical posts", "planned"),
        ],
        "rules": {
            "docs-quality.md": "# Documentation Quality Rules\n\n- All code examples in docs must be tested against the latest agamc.\n- Use consistent terminology aligned with the language specification.\n- Every API page must include at least one working example.\n- Keep navigation hierarchy flat: max 3 levels deep.\n",
        },
        "skills": {},
    },
}

# ============================================================
# GENERATE ALL FILES
# ============================================================

def write(path, content):
    full = os.path.join(BASE_DIR, path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, 'w', encoding='utf-8') as f:
        f.write(content)

for repo, cfg in REPOS.items():
    print(f"Generating multi-agent structure for {repo}...")
    
    # ---- Rich README ----
    crates_section = ""
    if cfg["crates"]:
        crates_section = "\n## Crates\n\n" + "\n".join([f"- `{c}/`" for c in cfg["crates"]]) + "\n"
    
    areas_section = "\n".join([f"- {a}" for a in cfg["areas"]])
    
    readme = f"""# {cfg['desc']}

> Part of the [agam-lang](https://github.com/agam-lang) organization.

## Mission

{cfg['mission']}

## Key Areas

{areas_section}
{crates_section}
## Status

This repository is under active development as part of the Agam ecosystem.

## Related Repositories

| Repository | Description |
|------------|-------------|
| [`agam`](https://github.com/agam-lang/agam) | Core compiler & toolchain |
| [`std`](https://github.com/agam-lang/std) | Standard library |
| [`agamlab`](https://github.com/agam-lang/agamlab) | Scientific computing platform |
| [`agam-vscode`](https://github.com/agam-lang/agam-vscode) | VS Code extension |
| [`rfcs`](https://github.com/agam-lang/rfcs) | Language design proposals |

## Contributing

Please see the organization-wide [Contributing Guide](https://github.com/agam-lang/.github/blob/main/CONTRIBUTING.md) and [Code of Conduct](https://github.com/agam-lang/.github/blob/main/CODE_OF_CONDUCT.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE).
"""
    write(f"{repo}/README.md", readme)

    # ---- LICENSE files ----
    write(f"{repo}/LICENSE-MIT", """MIT License

Copyright (c) 2024–2026 The Agam Authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
""")
    
    write(f"{repo}/LICENSE-APACHE", """                              Apache License
                        Version 2.0, January 2004
                     http://www.apache.org/licenses/

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
""")

    # ---- AGENTS.md ----
    write(f"{repo}/AGENTS.md", f"""# {cfg['desc']} — Agent Instructions

This file is the Codex entrypoint and universal repo entrypoint for shared agent guidance.
The canonical shared source of truth lives under `.agent/`.

## Read First

- `.agent/README.md`
- `.agent/include/project-context.md`
- `.agent/include/workflow.md`
- `.agent/phases/current.md`
- `.agent/rules/`

## Non-Negotiables

- This repository is part of the `agam-lang` organization ecosystem.
- All work must align with the Agam language vision: Python readability, Rust safety, native performance.
- Follow the coding and quality standards defined in `.agent/rules/`.
- Before closing a local slice, update `.agent/phases/current.md` and commit.

## Repo Map

- `.agent/`: canonical project guidance, rules, phases, skills, and policies
- `.github/`: CI/CD workflows and GitHub automation
- `README.md`: public-facing project description

## Multi-Agent Guidance

- Shared routing rules: `.agent/include/workflow.md`
- Role briefs and policies: `.agent/policy/`
""")

    # ---- CLAUDE.md ----
    write(f"{repo}/CLAUDE.md", f"""# {cfg['desc']} — Claude Instructions

Treat `.agent/` as the canonical shared source of truth for this repo.
Use `AGENTS.md` for the concise universal entrypoint, then fall through to the structured docs below.

## Read First

- `.agent/README.md`
- `.agent/include/project-context.md`
- `.agent/include/workflow.md`
- `.agent/phases/current.md`
- `.agent/rules/`

## Key Rules

- This is part of the Agam ecosystem. Align all work with the language's design principles.
- Follow the quality and process rules in `.agent/rules/`.
- Update `.agent/phases/current.md` when completing work.

## Tool-Specific Folders

- Shared guidance: `.agent/rules/`, `.agent/skills/`, `.agent/policy/`
""")

    # ---- .agent/README.md ----
    write(f"{repo}/.agent/README.md", f"""# {cfg['desc']} — Agent Board

This directory is the shared source of truth for agent-facing project guidance.
Use it as the canonical layer behind `AGENTS.md` and `CLAUDE.md`.

## Layout

- `include/`: compact shared context that all agents should read first.
- `rules/`: repo guardrails for structure, quality, and process.
- `policy/`: structured operating rules and project overview.
- `phases/`: compact phase-status board for what is active, complete, and next.
- `skills/`: repeatable project-specific workflows.

## Start Order

1. Read the root entrypoint for your client: `AGENTS.md` or `CLAUDE.md`.
2. Read `include/project-context.md` and `include/workflow.md`.
3. Read `phases/current.md` when deciding what to build next.
4. Read `policy/` for structured operating rules.
5. Read relevant files under `rules/`.
6. Use `skills/` when routing specialized work.

## Organization Context

This repository is part of the `agam-lang` GitHub organization.
The core compiler lives at `agam-lang/agam`.
Cross-repository architecture is documented in the `.github` repo's profile README.
""")

    # ---- .agent/include/project-context.md ----
    write(f"{repo}/.agent/include/project-context.md", f"""# Project Context

## What This Repository Is

- Repository: `agam-lang/{repo}`
- Purpose: {cfg['mission']}
- Technology: {cfg['tech']}

## Organization Context

- This is part of the `agam-lang` ecosystem.
- The core Agam compiler lives at `agam-lang/agam`.
- The standard library lives at `agam-lang/std`.
- The scientific platform lives at `agam-lang/agamlab`.

## Key Areas

{areas_section}

## Related Repositories

- Core compiler: https://github.com/agam-lang/agam
- Standard library: https://github.com/agam-lang/std
- Package registry: https://github.com/agam-lang/registry-index
- Documentation: https://github.com/agam-lang/agam-lang.github.io
""")

    # ---- .agent/include/workflow.md ----
    write(f"{repo}/.agent/include/workflow.md", f"""# Workflow

## Core Operating Rules

- Keep changes focused and well-scoped. Avoid unnecessary cross-file churn.
- All code must pass CI before merging to main.
- Route issues through GitHub Issues with appropriate labels.
- Before finalizing work, ensure documentation is updated.
- Follow the organization-wide contribution guidelines.

## Documentation Contract

- If you change public APIs, features, or workflows, update the README and any relevant docs.
- Keep agent-facing guidance synchronized through `.agent/` instead of adding divergent copies.
- Before closing a local implementation slice, record the change in `.agent/phases/current.md`.

## Delivery Style

- Prefer concise, structured work with concrete file ownership and verification notes.
- Test your changes before marking them complete.
- Link related issues and PRs across repositories when work spans the organization.
""")

    # ---- .agent/policy/README.md ----
    write(f"{repo}/.agent/policy/README.md", f"""# Policy

This folder contains structured operating rules and project context for `{repo}`.

## Files

- `project-overview.md`: what this repository is, why it exists, and the current direction

## Use

- Read `project-overview.md` when you need product and architecture context.
""")

    # ---- .agent/policy/project-overview.md ----
    write(f"{repo}/.agent/policy/project-overview.md", f"""# Project Overview

## What This Is

{cfg['mission']}

## Technology

{cfg['tech']}

## Key Areas

{areas_section}

## Organization Alignment

This repository is part of the `agam-lang` organization. All design decisions should align with:
- The Agam language vision (Python readability, Rust safety, native performance)
- The package ecosystem policy defined in `agam-lang/agam`
- The governance standards defined in `agam-lang/governance`
""")

    # ---- .agent/phases/current.md ----
    phase_lines = ""
    for i, (pid, name, desc, status) in enumerate(cfg["phases"], 1):
        phase_lines += f"{i}. **{pid}: {name}**\n   - Status: {status}\n   - Goal: {desc}\n\n"
    
    write(f"{repo}/.agent/phases/current.md", f"""# Current Development

## Active Workstreams

{phase_lines}
## Decision Rules

- Align all work with the Agam language vision and ecosystem policies.
- Follow the quality rules defined in `.agent/rules/`.
- Update this file when phase status changes.
""")

    # ---- .agent/rules/ ----
    for rule_name, rule_content in cfg.get("rules", {}).items():
        write(f"{repo}/.agent/rules/{rule_name}", rule_content)
    
    # Always add a common structure rule
    write(f"{repo}/.agent/rules/project-structure.md", f"""# Project Structure Rules

- Keep agent-facing guidance under `.agent/`; root `AGENTS.md` and `CLAUDE.md` are entrypoints, not competing sources of truth.
- All CI/CD configuration lives under `.github/workflows/`.
- Follow the organization-wide issue and PR templates from `.github`.
- Use consistent file naming and directory conventions across the organization.
""")

    # ---- .agent/skills/ ----
    for skill_name, skill_files in cfg.get("skills", {}).items():
        for fname, fcontent in skill_files.items():
            write(f"{repo}/.agent/skills/{skill_name}/{fname}", fcontent)

    # ---- .gitignore ----
    write(f"{repo}/.gitignore", """# Build artifacts
target/
dist/
build/
node_modules/
*.exe
*.dll
*.so
*.dylib

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Environment
.env
.env.local
""")

print("=" * 60)
print("Multi-agent structure generation complete for all repos!")
print("=" * 60)
