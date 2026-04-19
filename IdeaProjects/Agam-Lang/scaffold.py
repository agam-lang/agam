import os
import subprocess

BASE_DIR = r"c:\Users\ksvik\IdeaProjects\Agam-Lang"

files = {
    r".github\CODE_OF_CONDUCT.md": """# Contributor Covenant Code of Conduct

## Our Pledge

We as members, contributors, and leaders pledge to make participation in our
community a harassment-free experience for everyone, regardless of age, body
size, visible or invisible disability, ethnicity, sex characteristics, gender
identity and expression, level of experience, education, socio-economic status,
nationality, personal appearance, race, religion, or sexual identity
and orientation.
""",
    r".github\CONTRIBUTING.md": """# Contributing to Agam

Thank you for your interest in contributing to Agam! 

## Getting Started

1. Check the issue tracker
2. Read the RFCs
3. Submit a pull request
""",
    r".github\SECURITY.md": """# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

Please report security issues to security@agam-lang.org.
""",
    r".github\FUNDING.yml": """# These are supported funding model platforms
github: agam-lang
""",
    r".github\.github\ISSUE_TEMPLATE\bug_report.md": """---
name: Bug report
about: Create a report to help us improve
title: ''
labels: bug
assignees: ''

---

**Describe the bug**
A clear and concise description of what the bug is.

**Expected behavior**
A clear and concise description of what you expected to happen.
""",
    r".github\.github\ISSUE_TEMPLATE\feature_request.md": """---
name: Feature request
about: Suggest an idea for this project
title: ''
labels: enhancement
assignees: ''

---

**Is your feature request related to a problem? Please describe.**
A clear and concise description of what the problem is. Ex. I'm always frustrated when [...]

**Describe the solution you'd like**
A clear and concise description of what you want to happen.
""",
    r".github\.github\PULL_REQUEST_TEMPLATE.md": """## Description
Briefly describe the changes.

## Related Issues
Fixes #
""",
    r"agamlab\README.md": """# AGAMLAB

A MATLAB-like interactive scientific computing platform built on Agam.

## Features
- Interactive REPL
- Notebook interface
- Matrix/Linear Algebra
- Plotting
- Signal Processing
- Statistics
""",
    r"agamlab\LICENSE": """MIT License""",
    r"agamlab\Cargo.toml": """[workspace]
members = [
    "crates/*"
]
""",
    r"agamlab\agam.toml": """[workspace]
name = "agamlab"
version = "0.1.0"
""",
    r"agamlab\docs\architecture.md": """# Architecture\nAGAMLAB built on top of Agam.""",
    r"agamlab\docs\getting-started.md": """# Getting Started\nHow to use AGAMLAB.""",
    r"agamlab\docs\api-reference.md": """# API Reference\nAGAMLAB APIs.""",
    r"agamlab\examples\basic_matrix.agam": """@lang.advance
import agam_std.ndarray
fn main() -> i32 { return 0; }
""",
    r"agamlab\examples\signal_fft.agam": """@lang.advance
import agam_std.signal
fn main() -> i32 { return 0; }
""",
    r"agamlab\examples\linear_regression.agam": """@lang.advance
import agam_std.stats
fn main() -> i32 { return 0; }
""",
    r"agamlab\examples\data_visualization.agam": """@lang.advance
import agam_std.plot
fn main() -> i32 { return 0; }
""",
    r"agamlab\.github\workflows\ci.yml": """name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
""",
    r"agamlab\.github\workflows\release.yml": """name: Release
on:
  push:
    tags: ["v*"]
""",
    r"agamlab\.agent\README.md": """# Agent Context\nSpecific instructions for working in agamlab.""",
    r"agamlab\crates\agamlab_core\README.md": "# Core computation engine",
    r"agamlab\crates\agamlab_repl\README.md": "# Interactive REPL/console",
    r"agamlab\crates\agamlab_notebook\README.md": "# Notebook interface",
    r"agamlab\crates\agamlab_plot\README.md": "# Visualization & plotting",
    r"agamlab\crates\agamlab_matrix\README.md": "# Matrix operations",
    r"agamlab\crates\agamlab_signal\README.md": "# Signal processing",
    r"agamlab\crates\agamlab_stats\README.md": "# Statistical computing",
    r"agamlab\crates\agamlab_io\README.md": "# Data import/export",

    r"agam-lang.github.io\README.md": "# Agam Documentation Website\nPowered by mdBook or Docusaurus.",
    r"std\README.md": "# Agam Standard Library",
    r"registry-index\README.md": "# Agam Package Registry Index",
    r"sdk-packs\README.md": "# Agam SDK Bundles",
    r"rfcs\README.md": "# Agam RFCs",
    r"rfcs\0000-template.md": """# RFC Template
- Feature Name: 
- Start Date: 
""",
    r"agam-vscode\README.md": "# Agam VS Code Extension",
    r"agam-vscode\package.json": """{
  "name": "agam",
  "displayName": "Agam",
  "description": "Agam language support",
  "version": "0.1.0",
  "engines": { "vscode": "^1.74.0" }
}""",
    r"agam-intellij\README.md": "# Agam IntelliJ Plugin",
    r"playground\README.md": "# Agam Web Playground",
    r"examples\README.md": "# Agam Examples",
    r"benchmarks\README.md": "# Agam Benchmarks",
    r"governance\README.md": "# Agam Governance"
}

for path, content in files.items():
    full_path = os.path.join(BASE_DIR, path)
    os.makedirs(os.path.dirname(full_path), exist_ok=True)
    with open(full_path, 'w', encoding='utf-8') as f:
        f.write(content.strip() + "\n")
    print(f"Created {path}")

for repo in ['.github', 'agamlab', 'agam-lang.github.io', 'std', 'registry-index', 'sdk-packs', 'rfcs', 'agam-vscode', 'agam-intellij', 'playground', 'examples', 'benchmarks', 'governance']:
    repo_path = os.path.join(BASE_DIR, repo)
    if os.path.exists(repo_path):
        print(f"Initializing git in {repo}")
        subprocess.run(["git", "init"], cwd=repo_path, shell=True)

print("Scaffolding complete!")
