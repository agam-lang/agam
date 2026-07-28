# agam-ffi Python Package

Python-facing execution helpers built on top of the stable `agamc exec --json` contract.

This package keeps the default install small while exposing optional adapter hooks:

- core install: `AgamExecClient`, `AgamREPLTool`, and the strict request/response dataclasses
- optional `langchain` extra: `AgamREPLTool.to_langchain_structured_tool()`
- optional `llamaindex` extra: `AgamREPLTool.to_llamaindex_tool()`

Examples:

- `pip install agam-ffi[langchain]`
- `pip install agam-ffi[llamaindex]`
- `pip install agam-ffi[agent-frameworks]`

Usage:

```python
from agam_ffi import AgamREPLTool

tool = AgamREPLTool()
langchain_tool = tool.to_langchain_structured_tool()
llamaindex_tool = tool.to_llamaindex_tool()
```

The current adapter hooks were smoke-tested against live `langchain-core` and
`llama-index-core` installs via `uv run`.

Release workflow:

- local build/check: `uv run python -m build` and `uv run python -m twine check dist/*`
- GitHub release build/publish: `.github/workflows/agam-ffi-python.yml`
