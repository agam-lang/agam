//! Debug Adapter Protocol (DAP) Server & Session Handler.
//!
//! Manages client session state, source breakpoints, step-in/step-over execution flow,
//! thread stack frames, variable scopes, and Agam type pretty-printing.

use crate::dap::*;
use std::collections::HashMap;
use thiserror::Error;

/// DAP Server errors.
#[derive(Debug, Error)]
pub enum DapError {
    #[error("Unknown request command: {0}")]
    UnknownCommand(String),
    #[error("Invalid request arguments for command: {0}")]
    InvalidArguments(String),
    #[error("Target process is not running")]
    NotRunning,
    #[error("Breakpoint not found: {0}")]
    BreakpointNotFound(i64),
}

/// Target execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    Uninitialized,
    Configuring,
    Running,
    Stopped { reason: StopReason },
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Step,
    Breakpoint,
    Exception,
    Pause,
    Entry,
}

impl StopReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Step => "step",
            Self::Breakpoint => "breakpoint",
            Self::Exception => "exception",
            Self::Pause => "pause",
            Self::Entry => "entry",
        }
    }
}

/// Registered active breakpoint with hit counter and condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveBreakpoint {
    pub id: i64,
    pub source_path: String,
    pub line: i64,
    pub condition: Option<String>,
    pub hit_count: u64,
}

/// DAP Server session handling incoming requests and returning responses/events.
#[derive(Debug, Default)]
pub struct DapSession {
    pub seq_counter: i64,
    pub state: Option<ExecutionState>,
    pub next_breakpoint_id: i64,
    pub breakpoints: HashMap<String, Vec<ActiveBreakpoint>>,
    pub stack_frames: Vec<StackFrame>,
    pub variables: HashMap<i64, Vec<Variable>>,
}

impl DapSession {
    pub fn new() -> Self {
        Self {
            seq_counter: 1,
            state: Some(ExecutionState::Uninitialized),
            next_breakpoint_id: 1,
            breakpoints: HashMap::new(),
            stack_frames: Vec::new(),
            variables: HashMap::new(),
        }
    }

    fn next_seq(&mut self) -> i64 {
        let seq = self.seq_counter;
        self.seq_counter += 1;
        seq
    }

    /// Process an incoming DAP request and generate the appropriate response.
    pub fn handle_request(&mut self, req: Request) -> Result<Response, DapError> {
        match req.command.as_str() {
            "initialize" => self.handle_initialize(req),
            "launch" => self.handle_launch(req),
            "setBreakpoints" => self.handle_set_breakpoints(req),
            "configurationDone" => self.handle_configuration_done(req),
            "threads" => self.handle_threads(req),
            "stackTrace" => self.handle_stack_trace(req),
            "scopes" => self.handle_scopes(req),
            "variables" => self.handle_variables(req),
            "next" => self.handle_next(req),
            "stepIn" => self.handle_step_in(req),
            "continue" => self.handle_continue(req),
            "disconnect" => self.handle_disconnect(req),
            other => Err(DapError::UnknownCommand(other.to_string())),
        }
    }

    fn handle_initialize(&mut self, req: Request) -> Result<Response, DapError> {
        self.state = Some(ExecutionState::Configuring);
        let capabilities = Capabilities {
            supports_configuration_done_request: true,
            supports_conditional_breakpoints: true,
            supports_hit_conditional_breakpoints: true,
            supports_evaluate_for_hovers: true,
            supports_terminate_request: true,
            ..Default::default()
        };

        let seq = self.next_seq();
        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: Some(serde_json::to_value(capabilities).unwrap()),
        })
    }

    fn handle_launch(&mut self, req: Request) -> Result<Response, DapError> {
        self.state = Some(ExecutionState::Running);
        let seq = self.next_seq();
        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: None,
        })
    }

    fn handle_set_breakpoints(&mut self, req: Request) -> Result<Response, DapError> {
        let args = req
            .arguments
            .as_ref()
            .ok_or_else(|| DapError::InvalidArguments("setBreakpoints".into()))?;

        let source_path = args
            .get("source")
            .and_then(|s| s.get("path"))
            .and_then(|p| p.as_str())
            .unwrap_or("unknown")
            .to_string();

        let raw_breakpoints = args
            .get("breakpoints")
            .and_then(|b| b.as_array())
            .cloned()
            .unwrap_or_default();

        let mut verified_breakpoints = Vec::new();
        let mut active_list = Vec::new();

        for raw in raw_breakpoints {
            let line = raw.get("line").and_then(|l| l.as_i64()).unwrap_or(1);
            let condition = raw
                .get("condition")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());

            let bp_id = self.next_breakpoint_id;
            self.next_breakpoint_id += 1;

            active_list.push(ActiveBreakpoint {
                id: bp_id,
                source_path: source_path.clone(),
                line,
                condition,
                hit_count: 0,
            });

            verified_breakpoints.push(Breakpoint {
                id: Some(bp_id),
                verified: true,
                message: None,
                source: Some(Source {
                    name: None,
                    path: Some(source_path.clone()),
                    source_reference: None,
                }),
                line: Some(line),
                column: None,
            });
        }

        self.breakpoints.insert(source_path, active_list);

        let seq = self.next_seq();
        let body = serde_json::json!({
            "breakpoints": verified_breakpoints
        });

        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: Some(body),
        })
    }

    fn handle_configuration_done(&mut self, req: Request) -> Result<Response, DapError> {
        self.state = Some(ExecutionState::Running);
        let seq = self.next_seq();
        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: None,
        })
    }

    fn handle_threads(&mut self, req: Request) -> Result<Response, DapError> {
        let threads = vec![Thread {
            id: 1,
            name: "Main Thread (Agam Runtime)".into(),
        }];

        let seq = self.next_seq();
        let body = serde_json::json!({
            "threads": threads
        });

        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: Some(body),
        })
    }

    fn handle_stack_trace(&mut self, req: Request) -> Result<Response, DapError> {
        let seq = self.next_seq();
        let body = serde_json::json!({
            "stackFrames": self.stack_frames,
            "totalFrames": self.stack_frames.len()
        });

        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: Some(body),
        })
    }

    fn handle_scopes(&mut self, req: Request) -> Result<Response, DapError> {
        let scopes = vec![
            Scope {
                name: "Locals".into(),
                variables_reference: 1001,
                expensive: false,
                named_variables: Some(
                    self.variables
                        .get(&1001)
                        .map(|v| v.len() as i64)
                        .unwrap_or(0),
                ),
                indexed_variables: None,
            },
            Scope {
                name: "Globals".into(),
                variables_reference: 1002,
                expensive: true,
                named_variables: None,
                indexed_variables: None,
            },
        ];

        let seq = self.next_seq();
        let body = serde_json::json!({
            "scopes": scopes
        });

        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: Some(body),
        })
    }

    fn handle_variables(&mut self, req: Request) -> Result<Response, DapError> {
        let args = req
            .arguments
            .as_ref()
            .ok_or_else(|| DapError::InvalidArguments("variables".into()))?;

        let var_ref = args
            .get("variablesReference")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let vars = self.variables.get(&var_ref).cloned().unwrap_or_default();

        let seq = self.next_seq();
        let body = serde_json::json!({
            "variables": vars
        });

        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: Some(body),
        })
    }

    fn handle_next(&mut self, req: Request) -> Result<Response, DapError> {
        self.state = Some(ExecutionState::Stopped {
            reason: StopReason::Step,
        });
        let seq = self.next_seq();
        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: None,
        })
    }

    fn handle_step_in(&mut self, req: Request) -> Result<Response, DapError> {
        self.state = Some(ExecutionState::Stopped {
            reason: StopReason::Step,
        });
        let seq = self.next_seq();
        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: None,
        })
    }

    fn handle_continue(&mut self, req: Request) -> Result<Response, DapError> {
        self.state = Some(ExecutionState::Running);
        let seq = self.next_seq();
        let body = serde_json::json!({
            "allThreadsContinued": true
        });

        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: Some(body),
        })
    }

    fn handle_disconnect(&mut self, req: Request) -> Result<Response, DapError> {
        self.state = Some(ExecutionState::Terminated);
        let seq = self.next_seq();
        Ok(Response {
            seq,
            request_seq: req.seq,
            success: true,
            command: req.command,
            message: None,
            body: None,
        })
    }

    /// Set simulated stack frames and local variables for debugging inspect tests.
    pub fn set_frame_context(&mut self, frames: Vec<StackFrame>, locals: Vec<Variable>) {
        self.stack_frames = frames;
        self.variables.insert(1001, locals);
    }
}
