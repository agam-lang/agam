#!/usr/bin/env python3
"""Compatibility shim for the canonical devops SDK validation entrypoint."""

from __future__ import annotations

import runpy
from pathlib import Path


if __name__ == "__main__":
    runpy.run_path(
        str(Path(__file__).resolve().parents[1] / "devops" / "scripts" / "validate_sdk_e2e.py"),
        run_name="__main__",
    )
