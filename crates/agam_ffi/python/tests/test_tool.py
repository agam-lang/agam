import json
from pathlib import Path
import subprocess
import sys
import types
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from agam_ffi import (  # noqa: E402
    AgamExecClient,
    AgamExecError,
    AgamREPLTool,
    HeadlessExecutionPolicy,
    HeadlessExecutionRequest,
)


class AgamExecClientTests(unittest.TestCase):
    def test_run_request_invokes_exec_json_contract(self) -> None:
        client = AgamExecClient("agamc")
        request = HeadlessExecutionRequest(source="fn main() -> i32 { return 0; }")

        completed = subprocess.CompletedProcess(
            args=["agamc", "exec", "--json"],
            returncode=0,
            stdout=json.dumps(
                {
                    "success": True,
                    "filename": "snippet.agam",
                    "backend": "jit",
                    "exit_code": 0,
                    "stdout": "ok\n",
                    "stderr": "",
                }
            ),
            stderr="",
        )

        with mock.patch("agam_ffi.tool.subprocess.run", return_value=completed) as run_mock:
            response = client.run_request(request)

        run_mock.assert_called_once()
        args, kwargs = run_mock.call_args
        self.assertEqual(args[0], ["agamc", "exec", "--json"])
        self.assertEqual(json.loads(kwargs["input"])["source"], request.source)
        self.assertEqual(
            json.loads(kwargs["input"])["policy"]["max_source_bytes"], 1024 * 1024
        )
        self.assertEqual(json.loads(kwargs["input"])["policy"]["max_runtime_ms"], 30_000)
        self.assertEqual(
            json.loads(kwargs["input"])["policy"]["max_memory_bytes"],
            1024 * 1024 * 1024,
        )
        self.assertTrue(response.success)
        self.assertEqual(response.stdout, "ok\n")

    def test_run_request_raises_when_response_is_not_json(self) -> None:
        client = AgamExecClient("agamc")
        request = HeadlessExecutionRequest(source="fn main() -> i32 { return 0; }")

        completed = subprocess.CompletedProcess(
            args=["agamc", "exec", "--json"],
            returncode=1,
            stdout="not-json",
            stderr="broken",
        )

        with mock.patch("agam_ffi.tool.subprocess.run", return_value=completed):
            with self.assertRaises(AgamExecError) as error:
                client.run_request(request)

        self.assertEqual(error.exception.stdout, "not-json")
        self.assertEqual(error.exception.stderr, "broken")
        self.assertEqual(error.exception.status_code, 1)


class AgamReplToolTests(unittest.TestCase):
    def test_build_request_uses_tool_configuration(self) -> None:
        tool = AgamREPLTool(
            filename="agent.agam",
            backend="llvm",
            opt_level=3,
            fast=True,
            args=["alpha", "beta"],
            policy=HeadlessExecutionPolicy(max_source_bytes=2048),
        )

        request = tool.build_request("fn main() -> i32 { return 0; }")
        self.assertEqual(request.filename, "agent.agam")
        self.assertEqual(request.backend, "llvm")
        self.assertEqual(request.opt_level, 3)
        self.assertTrue(request.fast)
        self.assertEqual(request.args, ["alpha", "beta"])
        self.assertEqual(request.policy.max_source_bytes, 2048)
        self.assertEqual(request.policy.max_runtime_ms, 30_000)
        self.assertEqual(request.policy.max_memory_bytes, 1024 * 1024 * 1024)
        self.assertTrue(request.policy.allow_native_backends)

    def test_invoke_returns_stdout_when_execution_succeeds(self) -> None:
        tool = AgamREPLTool()
        response = mock.Mock(stdout="hi\n", error=None)

        with mock.patch.object(
            tool.client, "run_request", return_value=response
        ) as run_mock:
            output = tool.invoke("fn main(): println(\"hi\")")

        run_mock.assert_called_once()
        self.assertEqual(output, "hi\n")

    def test_to_langchain_structured_tool_uses_structured_tool_factory(self) -> None:
        tool = AgamREPLTool()
        response = mock.Mock(stdout="langchain\n", error=None)
        captured: dict[str, object] = {}

        class FakeStructuredTool:
            @classmethod
            def from_function(cls, func, **kwargs):
                captured["func"] = func
                captured["kwargs"] = kwargs
                return {"kind": "langchain", **kwargs}

        langchain_core = types.ModuleType("langchain_core")
        langchain_tools = types.ModuleType("langchain_core.tools")
        langchain_tools.StructuredTool = FakeStructuredTool
        langchain_core.tools = langchain_tools

        with mock.patch.dict(
            sys.modules,
            {
                "langchain_core": langchain_core,
                "langchain_core.tools": langchain_tools,
            },
        ):
            wrapped = tool.to_langchain_structured_tool(
                return_direct=True, metadata={"phase": 19}
            )

        self.assertEqual(wrapped["kind"], "langchain")
        self.assertEqual(captured["kwargs"]["name"], "agam_repl")
        self.assertTrue(captured["kwargs"]["return_direct"])
        with mock.patch.object(tool.client, "run_request", return_value=response):
            self.assertEqual(
                captured["func"]("fn main(): println(\"langchain\")"), "langchain\n"
            )

    def test_to_llamaindex_tool_uses_function_tool_factory(self) -> None:
        tool = AgamREPLTool()
        response = mock.Mock(stdout="llamaindex\n", error=None)
        captured: dict[str, object] = {}

        class FakeFunctionTool:
            @classmethod
            def from_defaults(cls, fn=None, **kwargs):
                captured["fn"] = fn
                captured["kwargs"] = kwargs
                return {"kind": "llamaindex", **kwargs}

        llama_index = types.ModuleType("llama_index")
        llama_index_core = types.ModuleType("llama_index.core")
        llama_index_tools = types.ModuleType("llama_index.core.tools")
        llama_index_tools.FunctionTool = FakeFunctionTool
        llama_index_core.tools = llama_index_tools
        llama_index.core = llama_index_core

        with mock.patch.dict(
            sys.modules,
            {
                "llama_index": llama_index,
                "llama_index.core": llama_index_core,
                "llama_index.core.tools": llama_index_tools,
            },
        ):
            wrapped = tool.to_llamaindex_tool(
                return_direct=True, metadata={"phase": 19}
            )

        self.assertEqual(wrapped["kind"], "llamaindex")
        self.assertEqual(captured["kwargs"]["name"], "agam_repl")
        self.assertTrue(captured["kwargs"]["return_direct"])
        with mock.patch.object(tool.client, "run_request", return_value=response):
            self.assertEqual(
                captured["fn"]("fn main(): println(\"llamaindex\")"), "llamaindex\n"
            )


if __name__ == "__main__":
    unittest.main()
