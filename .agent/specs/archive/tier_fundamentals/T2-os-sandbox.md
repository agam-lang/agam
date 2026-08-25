# Phase T2-os-sandbox — Chāṇakya Durdharṣa Sandboxing & Resource Bounds

## Phase Focus

OS-level process, memory, and capability isolation (`agam_runtime`) enforcing Chāṇakya Nīti Durdharṣa (sandboxing) and Kosha (resource treasury) principles for untrusted code execution.

## Key Capabilities

1. **OS-Native Hardening**:
   - **Win32 JobObjects**: Memory limits, process counts, UI isolation, and wall-clock timeout enforcement.
   - **Linux prctl/setrlimit/cgroups**: `PR_SET_NO_NEW_PRIVS`, CPU quota, RSS memory limits, and file descriptor bounds.

2. **Entropy & Subspace Resource Bounds**:
   - Bounded child process execution entropy ($H(Z \mid X, Y) \le \kappa + (\log_2 3)d$) ensuring untrusted code cannot leak host memory or exceed pre-allocated resource limits.

## Verification Plan

- Integration tests on Windows and Linux verifying process memory termination and wall-clock timeout enforcement.
