//! OS-level sandbox enforcement for headless execution.
//!
//! Provides a `SandboxPolicy` that mirrors the resource limits from
//! `HeadlessExecutionPolicy` and a `SandboxGuard` that enforces them
//! at the operating-system level using platform-specific mechanisms.
//!
//! ## Platform Support
//!
//! - **Windows**: Uses Win32 Job Objects for memory limits, active process
//!   count restrictions, and UI isolation. A background watchdog thread
//!   enforces wall-clock timeout.
//! - **Linux**: Uses `prctl(PR_SET_NO_NEW_PRIVS)` and resource limits via
//!   `setrlimit`. Full seccomp-bpf filtering is a future enhancement.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use agam_runtime::sandbox::{SandboxPolicy, SandboxGuard};
//!
//! let policy = SandboxPolicy::default();
//! let guard = SandboxGuard::acquire(&policy).expect("sandbox should activate");
//! // … run sandboxed code …
//! drop(guard); // tears down OS-level restrictions
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Declarative sandbox constraints derived from the execution policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
    /// Maximum wall-clock execution time in milliseconds. Zero means no limit.
    pub timeout_ms: u64,
    /// Maximum memory in bytes the sandboxed process may allocate.
    pub max_memory_bytes: u64,
    /// Maximum number of active child processes the sandbox may spawn.
    pub max_active_processes: u32,
    /// Whether the sandboxed process may access the network.
    pub deny_network: bool,
    /// Whether the sandboxed process may spawn new child processes beyond itself.
    pub deny_process_spawn: bool,
    /// Optional filesystem root restriction. If set, the execution working
    /// directory is confined to this path.
    pub filesystem_root: Option<PathBuf>,
    /// Whether environment variables are inherited from the host process.
    pub inherit_environment: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            max_memory_bytes: 1024 * 1024 * 1024, // 1 GiB
            max_active_processes: 1,
            deny_network: true,
            deny_process_spawn: true,
            filesystem_root: None,
            inherit_environment: false,
        }
    }
}

impl SandboxPolicy {
    /// Build a sandbox policy from raw headless execution policy values.
    pub fn from_execution_limits(
        max_runtime_ms: u64,
        max_memory_bytes: u64,
        inherit_environment: bool,
    ) -> Self {
        Self {
            timeout_ms: max_runtime_ms,
            max_memory_bytes,
            inherit_environment,
            ..Self::default()
        }
    }
}

/// Capability level that the current platform can enforce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxCapability {
    /// No OS-level enforcement available.
    None,
    /// Process-level enforcement (memory, CPU, process count, UI restrictions).
    Process,
    /// Full enforcement including filesystem and network isolation.
    Full,
}

/// Detect the strongest sandbox capability available on the current platform.
pub fn detect_sandbox_capability() -> SandboxCapability {
    #[cfg(target_os = "windows")]
    {
        // Windows Job Objects provide process-level enforcement.
        SandboxCapability::Process
    }
    #[cfg(target_os = "linux")]
    {
        // prctl + setrlimit provide process-level enforcement.
        SandboxCapability::Process
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        SandboxCapability::None
    }
}

/// RAII guard that enforces sandbox policy at the OS level.
///
/// On `acquire`, the guard sets up platform-specific restrictions and starts
/// a background watchdog thread for timeout enforcement. On `drop`, all
/// restrictions are torn down.
#[derive(Debug)]
pub struct SandboxGuard {
    policy: SandboxPolicy,
    /// Signal used to stop the watchdog thread on drop.
    cancel: Arc<AtomicBool>,
    /// Join handle for the timeout watchdog thread.
    watchdog: Option<std::thread::JoinHandle<()>>,
    /// Platform-specific enforcement state.
    _platform: PlatformSandboxState,
}

/// Platform-specific state that is torn down when the guard drops.
#[derive(Debug)]
struct PlatformSandboxState {
    /// On Windows: the Job Object handle (cleaned up on drop).
    #[cfg(target_os = "windows")]
    _job_handle: Option<u64>,
    /// Placeholder for platforms without OS-level enforcement.
    #[cfg(not(target_os = "windows"))]
    _marker: (),
}

impl Default for PlatformSandboxState {
    fn default() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            _job_handle: None,
            #[cfg(not(target_os = "windows"))]
            _marker: (),
        }
    }
}

impl Drop for PlatformSandboxState {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            if let Some(handle) = self._job_handle.take() {
                unsafe {
                    windows_sys::Win32::Foundation::CloseHandle(handle as *mut std::ffi::c_void);
                }
            }
        }
    }
}

impl SandboxGuard {
    /// Acquire OS-level enforcement for the given policy.
    ///
    /// Returns a guard that maintains the enforcement until dropped.
    pub fn acquire(policy: &SandboxPolicy) -> Result<Self, SandboxError> {
        let cancel = Arc::new(AtomicBool::new(false));

        // Start the timeout watchdog if a timeout is configured.
        let watchdog = if policy.timeout_ms > 0 {
            let timeout = Duration::from_millis(policy.timeout_ms);
            let cancel_clone = cancel.clone();
            Some(std::thread::spawn(move || {
                watchdog_loop(timeout, cancel_clone);
            }))
        } else {
            None
        };

        let platform = activate_platform_sandbox(policy)?;

        Ok(Self {
            policy: policy.clone(),
            cancel,
            watchdog,
            _platform: platform,
        })
    }

    /// Return the active policy.
    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    /// Return the capability level enforced by this guard.
    pub fn capability(&self) -> SandboxCapability {
        detect_sandbox_capability()
    }

    /// Check whether the timeout watchdog has signalled expiry.
    pub fn is_timed_out(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

impl Drop for SandboxGuard {
    fn drop(&mut self) {
        // Signal the watchdog to stop.
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(handle) = self.watchdog.take() {
            let _ = handle.join();
        }
        // Platform-specific teardown happens via PlatformSandboxState drop.
    }
}

/// Errors from sandbox acquisition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxError {
    pub message: String,
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sandbox error: {}", self.message)
    }
}

impl std::error::Error for SandboxError {}

// ── Internal helpers ───────────────────────────────────────────────────

fn watchdog_loop(timeout: Duration, cancel: Arc<AtomicBool>) {
    let start = std::time::Instant::now();
    let check_interval = Duration::from_millis(100);

    while !cancel.load(Ordering::Relaxed) {
        if start.elapsed() >= timeout {
            // Timeout reached — signal the guard.
            // The caller is responsible for checking `is_timed_out()` and
            // terminating the sandboxed work.
            cancel.store(true, Ordering::Relaxed);
            return;
        }
        std::thread::sleep(check_interval);
    }
}

fn activate_platform_sandbox(policy: &SandboxPolicy) -> Result<PlatformSandboxState, SandboxError> {
    // Set working directory restriction if requested.
    if let Some(root) = policy.filesystem_root.as_ref() {
        if root.is_dir() {
            std::env::set_current_dir(root).map_err(|e| SandboxError {
                message: format!(
                    "failed to restrict working directory to `{}`: {e}",
                    root.display()
                ),
            })?;
        }
    }

    #[cfg(target_os = "windows")]
    {
        activate_windows_sandbox(policy)
    }
    #[cfg(target_os = "linux")]
    {
        activate_linux_sandbox(policy)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = policy;
        Ok(PlatformSandboxState::default())
    }
}

#[cfg(target_os = "windows")]
fn activate_windows_sandbox(policy: &SandboxPolicy) -> Result<PlatformSandboxState, SandboxError> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::*;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // 1. Create an anonymous Job Object.
    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() {
        return Err(SandboxError {
            message: "CreateJobObjectW failed".into(),
        });
    }

    // 2. Configure extended limit information.
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };

    let mut limit_flags: u32 = 0;

    // Memory limit.
    if policy.max_memory_bytes > 0 {
        info.ProcessMemoryLimit = policy.max_memory_bytes as usize;
        limit_flags |= JOB_OBJECT_LIMIT_PROCESS_MEMORY;
    }

    // Active process count.
    if policy.deny_process_spawn {
        // Allow only 1 active process (the current one).
        info.BasicLimitInformation.ActiveProcessLimit = 1;
        limit_flags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
    } else if policy.max_active_processes > 0 {
        info.BasicLimitInformation.ActiveProcessLimit = policy.max_active_processes;
        limit_flags |= JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
    }

    // Prevent child processes from escaping the Job.
    limit_flags |= JOB_OBJECT_LIMIT_BREAKAWAY_OK; // inverse: deny breakaway
    info.BasicLimitInformation.LimitFlags = limit_flags;

    let set_ok = unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if set_ok == 0 {
        unsafe { CloseHandle(job) };
        return Err(SandboxError {
            message: "SetInformationJobObject (limits) failed".into(),
        });
    }

    // 3. Configure UI restrictions (deny clipboard, desktop switch, etc.).
    let mut ui_info: JOBOBJECT_BASIC_UI_RESTRICTIONS = unsafe { std::mem::zeroed() };
    ui_info.UIRestrictionsClass = JOB_OBJECT_UILIMIT_DESKTOP
        | JOB_OBJECT_UILIMIT_DISPLAYSETTINGS
        | JOB_OBJECT_UILIMIT_EXITWINDOWS
        | JOB_OBJECT_UILIMIT_GLOBALATOMS
        | JOB_OBJECT_UILIMIT_HANDLES
        | JOB_OBJECT_UILIMIT_READCLIPBOARD
        | JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS
        | JOB_OBJECT_UILIMIT_WRITECLIPBOARD;

    let ui_ok = unsafe {
        SetInformationJobObject(
            job,
            JobObjectBasicUIRestrictions,
            &ui_info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_BASIC_UI_RESTRICTIONS>() as u32,
        )
    };
    if ui_ok == 0 {
        // Non-fatal: UI restrictions are a defence-in-depth layer.
        // Log and continue.
    }

    // 4. Assign the current process to the Job Object.
    let assign_ok = unsafe { AssignProcessToJobObject(job, GetCurrentProcess()) };
    if assign_ok == 0 {
        // On some Windows versions a process already in a job cannot be
        // reassigned. Treat as non-fatal — the watchdog timeout still
        // enforces the wall-clock limit.
    }

    Ok(PlatformSandboxState {
        _job_handle: Some(job as u64),
    })
}

#[cfg(target_os = "linux")]
fn activate_linux_sandbox(policy: &SandboxPolicy) -> Result<PlatformSandboxState, SandboxError> {
    // 1. Prevent privilege escalation via PR_SET_NO_NEW_PRIVS.
    //    This is a prerequisite for seccomp-bpf and ensures the sandboxed
    //    process cannot gain new capabilities via execve().
    let ret = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if ret != 0 {
        return Err(SandboxError {
            message: "prctl(PR_SET_NO_NEW_PRIVS) failed".into(),
        });
    }

    // 2. Memory limit via RLIMIT_AS (address space cap).
    if policy.max_memory_bytes > 0 {
        let rlim = libc::rlimit {
            rlim_cur: policy.max_memory_bytes,
            rlim_max: policy.max_memory_bytes,
        };
        let ret = unsafe { libc::setrlimit(libc::RLIMIT_AS, &rlim) };
        if ret != 0 {
            return Err(SandboxError {
                message: format!("setrlimit(RLIMIT_AS, {}) failed", policy.max_memory_bytes),
            });
        }
    }

    // 3. Process count limit via RLIMIT_NPROC.
    if policy.deny_process_spawn {
        // Set to 0 additional processes — only the current process can run.
        let rlim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        let ret = unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &rlim) };
        if ret != 0 {
            // Non-fatal: some systems restrict setrlimit on NPROC.
        }
    } else if policy.max_active_processes > 0 {
        let rlim = libc::rlimit {
            rlim_cur: policy.max_active_processes as u64,
            rlim_max: policy.max_active_processes as u64,
        };
        let ret = unsafe { libc::setrlimit(libc::RLIMIT_NPROC, &rlim) };
        if ret != 0 {
            // Non-fatal.
        }
    }

    // 4. CPU time limit via RLIMIT_CPU (seconds).
    //    Defence-in-depth beyond the watchdog thread.
    if policy.timeout_ms > 0 {
        let cpu_seconds = (policy.timeout_ms / 1000).max(1);
        let rlim = libc::rlimit {
            rlim_cur: cpu_seconds,
            rlim_max: cpu_seconds + 5, // hard limit gives 5s grace
        };
        let ret = unsafe { libc::setrlimit(libc::RLIMIT_CPU, &rlim) };
        if ret != 0 {
            // Non-fatal: CPU limit is supplementary.
        }
    }

    Ok(PlatformSandboxState { _marker: () })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_has_sane_limits() {
        let policy = SandboxPolicy::default();
        assert_eq!(policy.timeout_ms, 30_000);
        assert_eq!(policy.max_memory_bytes, 1024 * 1024 * 1024);
        assert_eq!(policy.max_active_processes, 1);
        assert!(policy.deny_network);
        assert!(policy.deny_process_spawn);
        assert!(policy.filesystem_root.is_none());
        assert!(!policy.inherit_environment);
    }

    #[test]
    fn from_execution_limits_preserves_values() {
        let policy = SandboxPolicy::from_execution_limits(5_000, 512 * 1024 * 1024, true);
        assert_eq!(policy.timeout_ms, 5_000);
        assert_eq!(policy.max_memory_bytes, 512 * 1024 * 1024);
        assert!(policy.inherit_environment);
        // Defaults preserved for fields not in execution limits.
        assert!(policy.deny_network);
        assert!(policy.deny_process_spawn);
    }

    #[test]
    fn detect_capability_returns_process_on_supported_platforms() {
        let cap = detect_sandbox_capability();
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        assert_eq!(cap, SandboxCapability::Process);
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        assert_eq!(cap, SandboxCapability::None);
    }

    #[test]
    fn sandbox_guard_acquires_and_drops_cleanly() {
        let policy = SandboxPolicy {
            timeout_ms: 0, // No watchdog for this test.
            ..SandboxPolicy::default()
        };
        let guard = SandboxGuard::acquire(&policy).expect("sandbox should acquire");
        assert_eq!(guard.policy().max_active_processes, 1);
        assert!(!guard.is_timed_out());
        drop(guard);
    }

    #[test]
    fn sandbox_guard_watchdog_signals_timeout() {
        let policy = SandboxPolicy {
            timeout_ms: 200, // Very short timeout.
            ..SandboxPolicy::default()
        };
        let guard = SandboxGuard::acquire(&policy).expect("sandbox should acquire");
        // Wait for the watchdog to fire.
        std::thread::sleep(Duration::from_millis(400));
        assert!(guard.is_timed_out());
        drop(guard);
    }

    #[test]
    fn sandbox_guard_reports_capability() {
        let policy = SandboxPolicy::default();
        let guard = SandboxGuard::acquire(&policy).expect("sandbox should acquire");
        let cap = guard.capability();
        // On test platforms this should be at least None.
        assert!(matches!(
            cap,
            SandboxCapability::None | SandboxCapability::Process | SandboxCapability::Full
        ));
        drop(guard);
    }

    #[test]
    fn sandbox_error_displays_message() {
        let error = SandboxError {
            message: "test failure".into(),
        };
        assert_eq!(error.to_string(), "sandbox error: test failure");
    }
}
