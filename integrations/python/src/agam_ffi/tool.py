"""Thin Python wrappers around `agamc exec --json`."""

from __future__ import annotations

from dataclasses import dataclass, field
import json
import os
from pathlib import Path
import subprocess
from typing import Any, Iterable, Optional

AGAMC_EXECUTABLE_ENV = "AGAMC_EXECUTABLE"
DEFAULT_FILENAME = "snippet.agam"
DEFAULT_BACKEND = "jit"
DEFAULT_OPT_LEVEL = 2
DEFAULT_MAX_SOURCE_BYTES = 1024 * 1024
DEFAULT_MAX_ARG_COUNT = 64
DEFAULT_MAX_TOTAL_ARG_BYTES = 16 * 1024
DEFAULT_MAX_RUNTIME_MS = 30_000
DEFAULT_MAX_MEMORY_BYTES = 1024 * 1024 * 1024


@dataclass(slots=True)
class HeadlessExecutionPolicy:
    max_source_bytes: int = DEFAULT_MAX_SOURCE_BYTES
    max_arg_count: int = DEFAULT_MAX_ARG_COUNT
    max_total_arg_bytes: int = DEFAULT_MAX_TOTAL_ARG_BYTES
    max_runtime_ms: int = DEFAULT_MAX_RUNTIME_MS
    max_memory_bytes: int = DEFAULT_MAX_MEMORY_BYTES
    inherit_environment: bool = False
    allow_native_backends: bool = False

    def to_payload(self) -> dict[str, Any]:
        return {
            "max_source_bytes": self.max_source_bytes,
            "max_arg_count": self.max_arg_count,
            "max_total_arg_bytes": self.max_total_arg_bytes,
            "max_runtime_ms": self.max_runtime_ms,
            "max_memory_bytes": self.max_memory_bytes,
            "inherit_environment": self.inherit_environment,
            "allow_native_backends": self.allow_native_backends,
        }


@dataclass(slots=True)
class HeadlessExecutionRequest:
    source: str
    filename: str = DEFAULT_FILENAME
    args: list[str] = field(default_factory=list)
    backend: str = DEFAULT_BACKEND
    opt_level: int = DEFAULT_OPT_LEVEL
    fast: bool = False
    policy: HeadlessExecutionPolicy = field(default_factory=HeadlessExecutionPolicy)

    def to_payload(self) -> dict[str, Any]:
        return {
            "source": self.source,
            "filename": self.filename,
            "args": list(self.args),
            "backend": self.backend,
            "opt_level": self.opt_level,
            "fast": self.fast,
            "policy": self.policy.to_payload(),
        }


@dataclass(slots=True)
class HeadlessExecutionResponse:
    success: bool
    filename: str
    backend: str
    exit_code: Optional[int]
    stdout: str
    stderr: str
    error: Optional[str] = None

    @classmethod
    def from_payload(cls, payload: dict[str, Any]) -> "HeadlessExecutionResponse":
        return cls(
            success=bool(payload["success"]),
            filename=str(payload["filename"]),
            backend=str(payload["backend"]),
            exit_code=payload.get("exit_code"),
            stdout=str(payload.get("stdout", "")),
            stderr=str(payload.get("stderr", "")),
            error=payload.get("error"),
        )


class AgamExecError(RuntimeError):
    def __init__(
        self,
        message: str,
        *,
        stdout: str = "",
        stderr: str = "",
        status_code: Optional[int] = None,
    ) -> None:
        super().__init__(message)
        self.stdout = stdout
        self.stderr = stderr
        self.status_code = status_code


class AgamExecClient:
    """Thin subprocess client for `agamc exec --json`."""

    def __init__(self, executable: Optional[os.PathLike[str] | str] = None) -> None:
        if executable is None:
            executable = os.environ.get(AGAMC_EXECUTABLE_ENV, "agamc")
        self.executable = str(executable)

    def run_request(
        self, request: HeadlessExecutionRequest
    ) -> HeadlessExecutionResponse:
        payload = json.dumps(request.to_payload())
        result = subprocess.run(
            [self.executable, "exec", "--json"],
            input=payload,
            capture_output=True,
            text=True,
            check=False,
        )

        try:
            response_payload = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            raise AgamExecError(
                f"failed to parse `agamc exec` response: {error}",
                stdout=result.stdout,
                stderr=result.stderr,
                status_code=result.returncode,
            ) from error

        return HeadlessExecutionResponse.from_payload(response_payload)

    def run_source(
        self,
        source: str,
        *,
        filename: str = DEFAULT_FILENAME,
        backend: str = DEFAULT_BACKEND,
        opt_level: int = DEFAULT_OPT_LEVEL,
        fast: bool = False,
        args: Optional[Iterable[str]] = None,
        policy: Optional[HeadlessExecutionPolicy] = None,
    ) -> HeadlessExecutionResponse:
        if policy is None:
            policy = HeadlessExecutionPolicy(
                allow_native_backends=backend != DEFAULT_BACKEND
            )
        return self.run_request(
            HeadlessExecutionRequest(
                source=source,
                filename=filename,
                args=list(args or []),
                backend=backend,
                opt_level=opt_level,
                fast=fast,
                policy=policy,
            )
        )


@dataclass(slots=True)
class AgamREPLTool:
    """Python-facing execution tool for later agent-framework adapters."""

    client: AgamExecClient = field(default_factory=AgamExecClient)
    filename: str = DEFAULT_FILENAME
    backend: str = DEFAULT_BACKEND
    opt_level: int = DEFAULT_OPT_LEVEL
    fast: bool = False
    args: list[str] = field(default_factory=list)
    policy: HeadlessExecutionPolicy = field(default_factory=HeadlessExecutionPolicy)

    @property
    def name(self) -> str:
        return "agam_repl"

    @property
    def description(self) -> str:
        return "Execute Agam source through the stable `agamc exec --json` tool surface."

    def build_request(self, source: str) -> HeadlessExecutionRequest:
        policy = HeadlessExecutionPolicy(
            max_source_bytes=self.policy.max_source_bytes,
            max_arg_count=self.policy.max_arg_count,
            max_total_arg_bytes=self.policy.max_total_arg_bytes,
            max_runtime_ms=self.policy.max_runtime_ms,
            max_memory_bytes=self.policy.max_memory_bytes,
            inherit_environment=self.policy.inherit_environment,
            allow_native_backends=self.policy.allow_native_backends
            or self.backend != DEFAULT_BACKEND,
        )
        return HeadlessExecutionRequest(
            source=source,
            filename=self.filename,
            args=list(self.args),
            backend=self.backend,
            opt_level=self.opt_level,
            fast=self.fast,
            policy=policy,
        )

    def run(self, source: str) -> HeadlessExecutionResponse:
        return self.client.run_request(self.build_request(source))

    def invoke(self, source: str) -> str:
        response = self.run(source)
        if response.error:
            return response.error
        return response.stdout

    def to_langchain_structured_tool(
        self,
        *,
        name: Optional[str] = None,
        description: Optional[str] = None,
        return_direct: bool = False,
        **kwargs: Any,
    ) -> Any:
        """Create a LangChain StructuredTool backed by this Agam executor."""
        try:
            from langchain_core.tools import StructuredTool
        except ImportError as error:
            raise ImportError(
                "langchain-core is required for `AgamREPLTool.to_langchain_structured_tool()`"
            ) from error

        def run_agam_source(source: str) -> str:
            """Execute Agam source code and return stdout or a compiler/runtime error string."""
            return self.invoke(source)

        return StructuredTool.from_function(
            func=run_agam_source,
            name=name or self.name,
            description=description or self.description,
            return_direct=return_direct,
            **kwargs,
        )

    def to_llamaindex_tool(
        self,
        *,
        name: Optional[str] = None,
        description: Optional[str] = None,
        return_direct: bool = False,
        **kwargs: Any,
    ) -> Any:
        """Create a LlamaIndex FunctionTool backed by this Agam executor."""
        try:
            from llama_index.core.tools import FunctionTool
        except ImportError as error:
            raise ImportError(
                "llama-index-core is required for `AgamREPLTool.to_llamaindex_tool()`"
            ) from error

        def run_agam_source(source: str) -> str:
            """Execute Agam source code and return stdout or a compiler/runtime error string."""
            return self.invoke(source)

        return FunctionTool.from_defaults(
            fn=run_agam_source,
            name=name or self.name,
            description=description or self.description,
            return_direct=return_direct,
            **kwargs,
        )

    __call__ = invoke
