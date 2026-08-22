//! # agam_debug
//!
//! Debug Adapter Protocol (DAP) server, DWARF 5 / CodeView symbol metadata emitter,
//! and source-level debugging bridge for Agam-Lang.

pub mod dap;
pub mod dwarf;
pub mod server;

pub use dap::*;
pub use dwarf::*;
pub use server::{ActiveBreakpoint, DapError, DapSession, ExecutionState, StopReason};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dap_initialize_handshake() {
        let mut session = DapSession::new();
        let init_req = Request {
            seq: 1,
            command: "initialize".into(),
            arguments: Some(serde_json::json!({
                "clientID": "vscode",
                "adapterID": "agam-debug"
            })),
        };

        let response = session.handle_request(init_req).expect("Handle initialize");
        assert!(response.success);
        assert_eq!(response.command, "initialize");
        assert_eq!(response.request_seq, 1);
        assert_eq!(session.state, Some(ExecutionState::Configuring));

        let body = response.body.expect("Response body");
        let capabilities: Capabilities = serde_json::from_value(body).expect("Parse capabilities");
        assert!(capabilities.supports_configuration_done_request);
        assert!(capabilities.supports_conditional_breakpoints);
    }

    #[test]
    fn test_dap_breakpoint_registration_and_stack_inspection() {
        let mut session = DapSession::new();

        // 1. Set breakpoints
        let bp_req = Request {
            seq: 2,
            command: "setBreakpoints".into(),
            arguments: Some(serde_json::json!({
                "source": { "path": "src/main.agam" },
                "breakpoints": [
                    { "line": 42, "condition": "x > 10" },
                    { "line": 50 }
                ]
            })),
        };

        let bp_resp = session
            .handle_request(bp_req)
            .expect("Handle setBreakpoints");
        assert!(bp_resp.success);
        let bp_body = bp_resp.body.expect("BP body");
        let bps = bp_body
            .get("breakpoints")
            .and_then(|b| b.as_array())
            .expect("BPs array");
        assert_eq!(bps.len(), 2);
        assert_eq!(bps[0]["verified"], true);
        assert_eq!(bps[0]["line"], 42);

        // 2. Set simulated stack frame and variables
        let frames = vec![StackFrame {
            id: 100,
            name: "compute_eigenvalues".into(),
            source: Some(Source {
                name: Some("matrix.agam".into()),
                path: Some("src/matrix.agam".into()),
                source_reference: None,
            }),
            line: 128,
            column: 4,
            end_line: None,
            end_column: None,
            instruction_pointer_reference: Some("0x7fff12345678".into()),
        }];

        let vars = vec![
            Variable {
                name: "matrix_dim".into(),
                value: "1024".into(),
                ty: Some("i64".into()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                evaluate_name: Some("matrix_dim".into()),
            },
            Variable {
                name: "tolerance".into(),
                value: "0.000001".into(),
                ty: Some("f64".into()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
                evaluate_name: Some("tolerance".into()),
            },
        ];

        session.set_frame_context(frames, vars);

        // 3. Request stackTrace
        let stack_req = Request {
            seq: 3,
            command: "stackTrace".into(),
            arguments: None,
        };
        let stack_resp = session
            .handle_request(stack_req)
            .expect("Handle stackTrace");
        let stack_body = stack_resp.body.expect("Stack body");
        let stack_frames = stack_body
            .get("stackFrames")
            .and_then(|f| f.as_array())
            .expect("Frames");
        assert_eq!(stack_frames.len(), 1);
        assert_eq!(stack_frames[0]["name"], "compute_eigenvalues");

        // 4. Request variables
        let var_req = Request {
            seq: 4,
            command: "variables".into(),
            arguments: Some(serde_json::json!({ "variablesReference": 1001 })),
        };
        let var_resp = session.handle_request(var_req).expect("Handle variables");
        let var_body = var_resp.body.expect("Var body");
        let inspected_vars = var_body
            .get("variables")
            .and_then(|v| v.as_array())
            .expect("Vars");
        assert_eq!(inspected_vars.len(), 2);
        assert_eq!(inspected_vars[0]["name"], "matrix_dim");
        assert_eq!(inspected_vars[0]["value"], "1024");
    }

    #[test]
    fn test_dwarf_compilation_unit_generation() {
        let mut unit = DebugCompilationUnit::new("/workspace/agam_project");
        let file_idx = unit.add_file("main.agam", "src");
        assert_eq!(file_idx, 0);

        unit.add_line(LineEntry {
            address_offset: 0x1000,
            file_index: file_idx,
            line: 10,
            column: 1,
            is_stmt: true,
            is_prologue_end: true,
            is_epilogue_begin: false,
        });

        unit.add_subprogram(SubprogramEntry {
            name: "calculate_norm".into(),
            linkage_name: "_A14calculate_norm".into(),
            file_index: file_idx,
            start_line: 10,
            low_pc: 0x1000,
            high_pc: 0x1080,
            is_external: true,
            frame_base_reg: 7, // RBP / FP
            variables: vec![VariableLocationEntry {
                name: "vec_len".into(),
                type_name: "i64".into(),
                file_index: file_idx,
                line: 11,
                location: VariableLocation::StackOffset(-16),
            }],
        });

        assert_eq!(unit.line_table.len(), 1);
        assert_eq!(unit.subprograms.len(), 1);
        assert_eq!(unit.subprograms[0].variables.len(), 1);
    }
}
