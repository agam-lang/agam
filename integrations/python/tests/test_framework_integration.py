"""Integration tests that validate against live LangChain and LlamaIndex releases.

These tests are gated behind the optional extras (`pip install agam-ffi[agent-frameworks]`).
They verify that the adapter hooks produce valid tool instances compatible with the real
framework APIs, not just mock doubles.

Run with:
    pip install agam-ffi[agent-frameworks]
    python -m pytest tests/test_framework_integration.py -v

Tests that cannot import the framework are skipped automatically.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from agam_ffi import AgamREPLTool, HeadlessExecutionPolicy  # noqa: E402


def _has_langchain() -> bool:
    try:
        import langchain_core.tools  # noqa: F401
        return True
    except ImportError:
        return False


def _has_llamaindex() -> bool:
    try:
        import llama_index.core.tools  # noqa: F401
        return True
    except ImportError:
        return False


@unittest.skipUnless(_has_langchain(), "langchain-core is not installed")
class LangChainIntegrationTests(unittest.TestCase):
    """Validate AgamREPLTool.to_langchain_structured_tool() against live langchain-core."""

    def test_creates_valid_structured_tool(self) -> None:
        from langchain_core.tools import StructuredTool

        tool = AgamREPLTool(
            policy=HeadlessExecutionPolicy(max_source_bytes=4096),
        )
        lc_tool = tool.to_langchain_structured_tool(
            name="agam_exec",
            description="Run Agam source code",
        )

        self.assertIsInstance(lc_tool, StructuredTool)
        self.assertEqual(lc_tool.name, "agam_exec")
        self.assertEqual(lc_tool.description, "Run Agam source code")

    def test_structured_tool_has_source_input_schema(self) -> None:
        from langchain_core.tools import StructuredTool

        tool = AgamREPLTool()
        lc_tool = tool.to_langchain_structured_tool()

        self.assertIsInstance(lc_tool, StructuredTool)
        schema = lc_tool.args_schema
        if schema is not None:
            # The schema should describe a single `source` string parameter
            field_names = list(schema.model_fields.keys()) if hasattr(schema, 'model_fields') else []
            self.assertIn("source", field_names)

    def test_structured_tool_uses_custom_name_and_description(self) -> None:
        tool = AgamREPLTool()
        lc_tool = tool.to_langchain_structured_tool(
            name="custom_agam",
            description="Custom Agam runner",
            return_direct=True,
        )

        self.assertEqual(lc_tool.name, "custom_agam")
        self.assertEqual(lc_tool.description, "Custom Agam runner")
        self.assertTrue(lc_tool.return_direct)

    def test_default_name_falls_back_to_agam_repl(self) -> None:
        tool = AgamREPLTool()
        lc_tool = tool.to_langchain_structured_tool()

        self.assertEqual(lc_tool.name, "agam_repl")


@unittest.skipUnless(_has_llamaindex(), "llama-index-core is not installed")
class LlamaIndexIntegrationTests(unittest.TestCase):
    """Validate AgamREPLTool.to_llamaindex_tool() against live llama-index-core."""

    def test_creates_valid_function_tool(self) -> None:
        from llama_index.core.tools import FunctionTool

        tool = AgamREPLTool(
            policy=HeadlessExecutionPolicy(max_source_bytes=4096),
        )
        li_tool = tool.to_llamaindex_tool(
            name="agam_exec",
            description="Run Agam source code",
        )

        self.assertIsInstance(li_tool, FunctionTool)

    def test_function_tool_metadata_matches(self) -> None:
        tool = AgamREPLTool()
        li_tool = tool.to_llamaindex_tool(
            name="agam_runner",
            description="Execute Agam programs",
        )

        metadata = li_tool.metadata
        self.assertEqual(metadata.name, "agam_runner")
        self.assertEqual(metadata.description, "Execute Agam programs")

    def test_default_name_falls_back_to_agam_repl(self) -> None:
        tool = AgamREPLTool()
        li_tool = tool.to_llamaindex_tool()

        self.assertEqual(li_tool.metadata.name, "agam_repl")


class AdapterImportErrorTests(unittest.TestCase):
    """Verify that adapter hooks raise clear ImportErrors when frameworks are missing."""

    def test_langchain_raises_import_error_when_missing(self) -> None:
        # Temporarily remove langchain_core if present
        saved = {}
        keys_to_remove = [k for k in sys.modules if k.startswith("langchain")]
        for key in keys_to_remove:
            saved[key] = sys.modules.pop(key)

        try:
            import importlib
            # Create a fake import hook that blocks langchain
            original_import = __builtins__.__import__ if hasattr(__builtins__, '__import__') else __import__

            def blocking_import(name, *args, **kwargs):
                if name.startswith("langchain"):
                    raise ImportError(f"No module named '{name}'")
                return original_import(name, *args, **kwargs)

            with unittest.mock.patch("builtins.__import__", side_effect=blocking_import):
                tool = AgamREPLTool()
                with self.assertRaises(ImportError) as ctx:
                    tool.to_langchain_structured_tool()
                self.assertIn("langchain-core", str(ctx.exception))
        finally:
            sys.modules.update(saved)

    def test_llamaindex_raises_import_error_when_missing(self) -> None:
        saved = {}
        keys_to_remove = [k for k in sys.modules if k.startswith("llama_index")]
        for key in keys_to_remove:
            saved[key] = sys.modules.pop(key)

        try:
            original_import = __builtins__.__import__ if hasattr(__builtins__, '__import__') else __import__

            def blocking_import(name, *args, **kwargs):
                if name.startswith("llama_index"):
                    raise ImportError(f"No module named '{name}'")
                return original_import(name, *args, **kwargs)

            with unittest.mock.patch("builtins.__import__", side_effect=blocking_import):
                tool = AgamREPLTool()
                with self.assertRaises(ImportError) as ctx:
                    tool.to_llamaindex_tool()
                self.assertIn("llama-index-core", str(ctx.exception))
        finally:
            sys.modules.update(saved)


if __name__ == "__main__":
    unittest.main()
