# Graph Report - agam  (2026-04-17)

## Corpus Check
- 170 files · ~204,859 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 2687 nodes · 5850 edges · 87 communities detected
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 212 edges (avg confidence: 0.75)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Compiler Driver|Compiler Driver]]
- [[_COMMUNITY_Package Manager|Package Manager]]
- [[_COMMUNITY_Benchmarks|Benchmarks]]
- [[_COMMUNITY_JIT Engine|JIT Engine]]
- [[_COMMUNITY_LLVM Codegen|LLVM Codegen]]
- [[_COMMUNITY_Automatic Differentiation|Automatic Differentiation]]
- [[_COMMUNITY_Expression Parser|Expression Parser]]
- [[_COMMUNITY_SIMD Operations|SIMD Operations]]
- [[_COMMUNITY_Call Cache Admission|Call Cache Admission]]
- [[_COMMUNITY_Math Primitives|Math Primitives]]
- [[_COMMUNITY_IR Lowering Pipeline|IR Lowering Pipeline]]
- [[_COMMUNITY_DataFrames|DataFrames]]
- [[_COMMUNITY_Machine Learning|Machine Learning]]
- [[_COMMUNITY_C Code Generator|C Code Generator]]
- [[_COMMUNITY_Lexer|Lexer]]
- [[_COMMUNITY_Ownership System|Ownership System]]
- [[_COMMUNITY_Tensors|Tensors]]
- [[_COMMUNITY_N-Dimensional Arrays|N-Dimensional Arrays]]
- [[_COMMUNITY_Complex Numbers|Complex Numbers]]
- [[_COMMUNITY_Build Cache|Build Cache]]
- [[_COMMUNITY_Effect Handlers|Effect Handlers]]
- [[_COMMUNITY_Constant Evaluator|Constant Evaluator]]
- [[_COMMUNITY_Type System|Type System]]
- [[_COMMUNITY_Smart Pointers|Smart Pointers]]
- [[_COMMUNITY_Statistics|Statistics]]
- [[_COMMUNITY_Declaration AST|Declaration AST]]
- [[_COMMUNITY_Language Server|Language Server]]
- [[_COMMUNITY_Hardware Info|Hardware Info]]
- [[_COMMUNITY_Source Locations|Source Locations]]
- [[_COMMUNITY_Loop Optimizer|Loop Optimizer]]
- [[_COMMUNITY_Lexer Cursor|Lexer Cursor]]
- [[_COMMUNITY_Call Cache Analyzer|Call Cache Analyzer]]
- [[_COMMUNITY_Name Resolver|Name Resolver]]
- [[_COMMUNITY_Type Inference|Type Inference]]
- [[_COMMUNITY_Error Diagnostics|Error Diagnostics]]
- [[_COMMUNITY_Scope Manager|Scope Manager]]
- [[_COMMUNITY_Numeric Methods|Numeric Methods]]
- [[_COMMUNITY_Type Checker|Type Checker]]
- [[_COMMUNITY_Method Dispatch|Method Dispatch]]
- [[_COMMUNITY_Code Formatter|Code Formatter]]
- [[_COMMUNITY_Diagnostic Emitter|Diagnostic Emitter]]
- [[_COMMUNITY_Pattern Matching|Pattern Matching]]
- [[_COMMUNITY_SMT Solver|SMT Solver]]
- [[_COMMUNITY_ODE and Optimization Solvers|ODE and Optimization Solvers]]
- [[_COMMUNITY_Function Inliner|Function Inliner]]
- [[_COMMUNITY_Package Loader|Package Loader]]
- [[_COMMUNITY_MIR Nodes|MIR Nodes]]
- [[_COMMUNITY_Dead Code Eliminator|Dead Code Eliminator]]
- [[_COMMUNITY_HIR Nodes|HIR Nodes]]
- [[_COMMUNITY_Expression AST|Expression AST]]
- [[_COMMUNITY_Pretty Printer|Pretty Printer]]
- [[_COMMUNITY_Constant Folder|Constant Folder]]
- [[_COMMUNITY_Bench — Edit Distance|Bench — Edit Distance]]
- [[_COMMUNITY_Bench — Polynomial|Bench — Polynomial]]
- [[_COMMUNITY_Bench — Ring Buffer|Bench — Ring Buffer]]
- [[_COMMUNITY_Bench — CSV Scan|Bench — CSV Scan]]
- [[_COMMUNITY_Escape Analysis|Escape Analysis]]
- [[_COMMUNITY_Verification Cache|Verification Cache]]
- [[_COMMUNITY_Bench — Fibonacci|Bench — Fibonacci]]
- [[_COMMUNITY_Bench — Matrix Multiply|Bench — Matrix Multiply]]
- [[_COMMUNITY_Bench — Tensor MatMul|Bench — Tensor MatMul]]
- [[_COMMUNITY_Type Expression AST|Type Expression AST]]
- [[_COMMUNITY_Runtime Builtins|Runtime Builtins]]
- [[_COMMUNITY_Benchmark Config|Benchmark Config]]
- [[_COMMUNITY_Statement AST|Statement AST]]
- [[_COMMUNITY_Symbol Table|Symbol Table]]
- [[_COMMUNITY_Pattern AST|Pattern AST]]
- [[_COMMUNITY_Parser Core|Parser Core]]
- [[_COMMUNITY_MIR Optimizer|MIR Optimizer]]
- [[_COMMUNITY_AST Visitor|AST Visitor]]
- [[_COMMUNITY_Tokenizer|Tokenizer]]
- [[_COMMUNITY_Codegen|Codegen]]
- [[_COMMUNITY_Debugger|Debugger]]
- [[_COMMUNITY_Doc Generator|Doc Generator]]
- [[_COMMUNITY_Error Framework|Error Framework]]
- [[_COMMUNITY_Foreign Function Interface|Foreign Function Interface]]
- [[_COMMUNITY_Game Engine|Game Engine]]
- [[_COMMUNITY_HIR|HIR]]
- [[_COMMUNITY_Linter|Linter]]
- [[_COMMUNITY_Macro System|Macro System]]
- [[_COMMUNITY_MIR|MIR]]
- [[_COMMUNITY_Notebooks|Notebooks]]
- [[_COMMUNITY_Runtime|Runtime]]
- [[_COMMUNITY_Semantic Analysis|Semantic Analysis]]
- [[_COMMUNITY_Formal Verification|Formal Verification]]
- [[_COMMUNITY_Standard Library|Standard Library]]
- [[_COMMUNITY_UI Framework|UI Framework]]

## God Nodes (most connected - your core abstractions)
1. `main()` - 63 edges
2. `temp_dir()` - 59 edges
3. `temp_dir()` - 42 edges
4. `Parser` - 41 edges
5. `scaffold_workspace_manifest()` - 31 edges
6. `TypeStore` - 26 edges
7. `BenchmarkWorkspace` - 24 edges
8. `prewarm_daemon_entry_artifacts()` - 24 edges
9. `write_workspace_manifest_to_path()` - 24 edges
10. `sample_source()` - 24 edges

## Surprising Connections (you probably didn't know these)
- `Runtime memory and static space-profile collection.` --uses--> `CpuTopology`  [INFERRED]
  agam\benchmarks\infrastructure\memory_profiler.py → agam\benchmarks\infrastructure\cpu_profiler.py
- `main()` --calls--> `editDistanceCost()`  [EXTRACTED]
  agam\benchmarks\benchmarks\01_algorithms\comparisons\edit_distance.rs → agam\benchmarks\benchmarks\01_algorithms\comparisons\edit_distance.go
- `main()` --calls--> `polynomialCost()`  [EXTRACTED]
  agam\benchmarks\benchmarks\02_numerical_computation\comparisons\polynomial_eval.rs → agam\benchmarks\benchmarks\02_numerical_computation\comparisons\polynomial_eval.go
- `main()` --calls--> `ringBufferCost()`  [EXTRACTED]
  agam\benchmarks\benchmarks\03_data_structures\comparisons\ring_buffer.rs → agam\benchmarks\benchmarks\03_data_structures\comparisons\ring_buffer.go
- `main()` --calls--> `csvScanCost()`  [EXTRACTED]
  agam\benchmarks\benchmarks\07_io_operations\comparisons\csv_scanning.rs → agam\benchmarks\benchmarks\07_io_operations\comparisons\csv_scanning.go

## Communities

### Community 0 - "Compiler Driver"
Cohesion: 0.01
Nodes (415): active_daemon_status(), allow_dev_wsl_llvm(), android_ndk_host_tag(), apply_persisted_optimize_profile(), apply_persisted_specialization_profile(), audit_registry_index_package(), Backend, backend_from_runtime_backend() (+407 more)

### Community 1 - "Package Manager"
Cohesion: 0.02
Nodes (203): append_release_immutability(), append_release_to_index(), audit_registry_package(), audit_registry_package_output(), build_portable_package(), builds_package_with_verified_ir_and_source_map(), classify_dependency_source(), collect_agam_files() (+195 more)

### Community 2 - "Benchmarks"
Cohesion: 0.03
Nodes (94): AgamHarness, _is_windows(), BaseHarness, PreparedBenchmark, BaseHarness, build_parser(), main(), BenchmarkWorkspace (+86 more)

### Community 3 - "JIT Engine"
Cohesion: 0.03
Nodes (107): AgamJit, analyze_call_cache(), analyze_function(), analyze_module(), binop_operand_type(), build_call_cache_analysis(), build_specialization_registry(), builtin_arg_types() (+99 more)

### Community 4 - "LLVM Codegen"
Cohesion: 0.04
Nodes (117): analyze_call_cache(), analyze_function(), analyze_function_attrs(), analyze_module(), bool_const_sign(), build_call_cache_analysis(), build_specialization_registry(), builtin_arg_types() (+109 more)

### Community 5 - "Automatic Differentiation"
Cohesion: 0.04
Nodes (46): Dual, GradTape, TapeNode, test_dual_add(), test_dual_chain_rule(), test_dual_constant(), test_dual_exp(), test_dual_mul() (+38 more)

### Community 6 - "Expression Parser"
Cohesion: 0.14
Nodes (24): Assoc, parse_expr(), parse_src(), Parser, test_array_literal(), test_binary_add(), test_chained_method(), test_comparison() (+16 more)

### Community 7 - "SIMD Operations"
Cohesion: 0.08
Nodes (42): AlignmentHint, axpy_inplace(), axpy_neon(), axpy_x86_avx(), axpy_x86_avx512(), axpy_x86_sse2(), BinaryOp, dispatch_binary() (+34 more)

### Community 8 - "Call Cache Admission"
Cohesion: 0.06
Nodes (52): adaptive_admission_accepts_repeated_short_reuse(), adaptive_admission_decision(), adaptive_admission_decision_with_specialization_feedback(), adaptive_admission_neutral_clone_feedback_blocks_unfavorable_penalty(), adaptive_admission_penalizes_unfavorable_specialization_feedback(), adaptive_admission_rejects_unique_inputs(), adaptive_admission_uses_favorable_clone_feedback_over_neutral_aggregate(), adaptive_admission_uses_favorable_specialization_feedback_in_optimize_mode() (+44 more)

### Community 9 - "Math Primitives"
Cohesion: 0.06
Nodes (24): agam_adam(), agam_adam(), BigUint, Interval, test_biguint_add(), test_biguint_factorial(), test_biguint_factorial_25(), test_biguint_from_u64() (+16 more)

### Community 10 - "IR Lowering Pipeline"
Cohesion: 0.09
Nodes (22): HirExprKind, HirLowering, lower_binop(), lower_source(), lower_to_mir(), lower_unaryop(), lower_unop(), MirLowering (+14 more)

### Community 11 - "DataFrames"
Cohesion: 0.08
Nodes (14): Column, DataFrame, sample_df(), test_df_column_access(), test_df_describe(), test_df_filter(), test_df_group_by_sum(), test_df_head_tail() (+6 more)

### Community 12 - "Machine Learning"
Cohesion: 0.06
Nodes (29): batch_norm(), cross_entropy(), dense(), euclidean_distance(), f1_score(), gelu(), huber_loss(), knn_classify() (+21 more)

### Community 13 - "C Code Generator"
Cohesion: 0.09
Nodes (38): analyze_function(), analyze_module(), binop_to_c(), builtin_signature(), BuiltinSig, compile_to_c(), CType, emit_block() (+30 more)

### Community 14 - "Lexer"
Cohesion: 0.08
Nodes (18): is_ident_start(), kinds(), lex(), Lexer, Lexer<'src>, SyntaxMode, test_block_comment(), test_doc_comment() (+10 more)

### Community 15 - "Ownership System"
Cohesion: 0.14
Nodes (22): ActiveBorrow, BorrowKind, dummy_span(), MemoryMode, OwnershipError, OwnershipState, OwnershipTracker, test_arc_assign_to_immutable_ok() (+14 more)

### Community 16 - "Tensors"
Cohesion: 0.09
Nodes (15): Tensor, test_add(), test_dot(), test_matmul(), test_mean(), test_mul(), test_ones(), test_relu() (+7 more)

### Community 17 - "N-Dimensional Arrays"
Cohesion: 0.08
Nodes (29): arange(), clip(), concatenate(), cumsum(), diag(), eye(), flatten(), linspace() (+21 more)

### Community 18 - "Complex Numbers"
Cohesion: 0.1
Nodes (11): Complex, Quaternion, test_complex_add(), test_complex_div(), test_complex_exp(), test_complex_mul(), test_complex_powi(), test_complex_sqrt() (+3 more)

### Community 19 - "Build Cache"
Cohesion: 0.12
Nodes (19): cache_root_for_path(), CacheArtifactKind, CacheEntry, CacheHit, CacheKey, CacheStatus, CacheStatusByKind, CacheStatusEntry (+11 more)

### Community 20 - "Effect Handlers"
Cohesion: 0.14
Nodes (20): CpsNode, CpsNodeKind, EffectDef, EffectOpDef, EffectRegistry, HandlerClauseDef, HandlerDef, make_io_effect() (+12 more)

### Community 21 - "Constant Evaluator"
Cohesion: 0.16
Nodes (26): binop(), bool_expr(), ConstEvalError, ConstEvaluator, ConstValue, float_expr(), int_expr(), str_expr() (+18 more)

### Community 22 - "Type System"
Cohesion: 0.11
Nodes (5): builtin_type_id_for_name(), FloatSize, IntSize, Type, TypeStore

### Community 23 - "Smart Pointers"
Cohesion: 0.1
Nodes (13): AgamArc, AgamArc<T>, AgamWeak, ArcHeader, ArcInner, test_agam_arc_cache_line_alignment(), test_agam_arc_retain_release(), test_agam_arc_simd_alignment_hint() (+5 more)

### Community 24 - "Statistics"
Cohesion: 0.12
Nodes (7): Rng, Stats, test_rng_normal_mean(), test_rng_range(), test_rng_uniform(), test_welch_t_test(), welch_t_test()

### Community 25 - "Declaration AST"
Cohesion: 0.08
Nodes (23): Annotation, Attribute, Decl, DeclKind, EffectDecl, EffectOp, EnumDecl, EnumVariant (+15 more)

### Community 26 - "Language Server"
Cohesion: 0.16
Nodes (23): apply_did_change(), apply_did_close(), apply_did_open(), decode_hex_digit(), document_end_position(), error_response(), file_uri(), format_document() (+15 more)

### Community 27 - "Hardware Info"
Cohesion: 0.12
Nodes (13): CacheInfo, Endianness, HardwareInfo, hwinfo(), SimdCapabilities, SimdTier, test_cache_defaults(), test_endianness() (+5 more)

### Community 28 - "Source Locations"
Cohesion: 0.15
Nodes (10): sample_file(), SourceFile, SourceId, Span, test_dummy_span(), test_line_col_first_line(), test_line_col_second_line(), test_line_text() (+2 more)

### Community 29 - "Loop Optimizer"
Cohesion: 0.17
Nodes (19): analyze_condition(), analyze_loop(), clone_op(), collect_predecessors(), compute_trip_count(), find_initial_value(), find_step(), flip_cmp() (+11 more)

### Community 30 - "Lexer Cursor"
Cohesion: 0.14
Nodes (7): Cursor, Cursor<'src>, test_basic_advance(), test_eat_while(), test_peek(), test_slice_from(), test_utf8()

### Community 31 - "Call Cache Analyzer"
Cohesion: 0.15
Nodes (17): analyze_call_cache(), builtin_call_cache_semantics(), BuiltinCallCacheSemantics, CallCacheAnalysis, CallCacheFunctionAnalysis, CallCacheMode, CallCacheRejectReason, CallCacheRequest (+9 more)

### Community 32 - "Name Resolver"
Cohesion: 0.2
Nodes (7): parse_and_resolve(), ResolveError, Resolver, test_for_loop_variable(), test_function_params_in_scope(), test_resolve_simple_function(), test_undeclared_variable()

### Community 33 - "Type Inference"
Cohesion: 0.23
Nodes (12): Constraint, InferenceEngine, InferenceError, test_any_unifies_with_everything(), test_concrete_type_mismatch(), test_function_arity_mismatch(), test_function_type_unification(), test_ref_mutability_mismatch() (+4 more)

### Community 34 - "Error Diagnostics"
Cohesion: 0.14
Nodes (7): Diagnostic, DiagnosticLevel, ErrorCode, Label, test_error_creation(), test_ice_is_error(), test_warning_is_not_error()

### Community 35 - "Scope Manager"
Cohesion: 0.23
Nodes (8): dummy_span(), Scope, ScopeStack, test_declare_and_lookup(), test_mark_used(), test_nested_scopes(), test_redeclaration_in_same_scope_errors(), test_shadowing_across_scopes()

### Community 36 - "Numeric Methods"
Cohesion: 0.15
Nodes (13): bisect(), fft(), ifft(), integrate_gauss5(), integrate_simpson(), newton(), test_bisect_sqrt2(), test_fft_ifft_roundtrip() (+5 more)

### Community 37 - "Type Checker"
Cohesion: 0.22
Nodes (9): check_source(), test_arithmetic_same_type(), test_comparison_returns_bool(), test_if_requires_bool(), test_let_int_literal(), test_logical_and_requires_bool(), test_while_requires_bool(), TypeChecker (+1 more)

### Community 38 - "Method Dispatch"
Cohesion: 0.24
Nodes (11): dummy_span(), ImplEntry, MethodSig, test_coherence_rejects_duplicate_impl(), test_inherent_before_trait(), test_missing_method_check(), test_register_and_resolve_inherent_method(), test_trait_method_resolution() (+3 more)

### Community 39 - "Code Formatter"
Cohesion: 0.21
Nodes (15): expands_leading_tabs_without_touching_inline_tabs(), format_inputs(), format_inputs_uses_workspace_expansion(), format_path(), format_paths(), format_paths_writes_changed_files(), format_source(), format_source_with_options() (+7 more)

### Community 40 - "Diagnostic Emitter"
Cohesion: 0.22
Nodes (4): DiagnosticEmitter, test_emit_with_label(), test_emitter_counts(), test_emitter_no_errors()

### Community 41 - "Pattern Matching"
Cohesion: 0.3
Nodes (14): check_exhaustiveness(), dummy_span(), ExhaustivenessError, SimplePattern, test_bool_exhaustive(), test_bool_missing_false(), test_bool_missing_true(), test_duplicate_pattern() (+6 more)

### Community 42 - "SMT Solver"
Cohesion: 0.22
Nodes (6): Constraint, SmtSolver, SolverResult, test_mock_solver_div_zero_proof(), test_smtlib_format(), Z3Solver

### Community 43 - "ODE and Optimization Solvers"
Cohesion: 0.22
Nodes (11): adam(), linear_regression(), rk4(), rk4_system(), test_adam_2d(), test_adam_quadratic(), test_linear_regression(), test_linear_regression_noisy() (+3 more)

### Community 44 - "Function Inliner"
Cohesion: 0.33
Nodes (9): inlines_small_leaf_calls(), InlineState, is_inline_candidate(), optimize_source(), reachable_blocks(), remap_local(), remap_op(), remap_value() (+1 more)

### Community 45 - "Package Loader"
Cohesion: 0.24
Nodes (12): host_runtime(), PackageLoadPlan, plan_package_load(), plan_package_load_rejects_host_native_mismatch(), plan_package_load_uses_manifest_backend_for_auto(), portable_manifest_uses_current_abi_and_host(), portable_runtime_manifest(), RuntimeAbi (+4 more)

### Community 46 - "MIR Nodes"
Cohesion: 0.17
Nodes (11): BasicBlock, BlockId, Instruction, MirBinOp, MirFunction, MirModule, MirParam, MirUnOp (+3 more)

### Community 47 - "Dead Code Eliminator"
Cohesion: 0.32
Nodes (11): collect_live_locals(), dce_function(), is_side_effectful(), mark_used_values(), optimize_source(), reachable_blocks(), removes_dead_locals(), removes_unreachable_blocks_after_folded_branch() (+3 more)

### Community 48 - "HIR Nodes"
Cohesion: 0.18
Nodes (10): HirBinOp, HirBlock, HirExpr, HirExprKind, HirFunction, HirId, HirModule, HirParam (+2 more)

### Community 49 - "Expression AST"
Cohesion: 0.2
Nodes (9): BinOp, Block, Expr, ExprKind, FieldInit, FStringPart, LambdaParam, MatchArm (+1 more)

### Community 50 - "Pretty Printer"
Cohesion: 0.44
Nodes (2): pretty_print(), PrettyPrinter

### Community 51 - "Constant Folder"
Cohesion: 0.36
Nodes (8): Constant, fold_binop(), fold_unop(), folds_constant_branches_into_jumps(), folds_literal_arithmetic(), optimize_source(), propagates_constants_through_locals(), run()

### Community 52 - "Bench — Edit Distance"
Cohesion: 0.39
Nodes (3): edit_distance_cost(), editDistanceCost(), main()

### Community 53 - "Bench — Polynomial"
Cohesion: 0.39
Nodes (3): main(), polynomial_cost(), polynomialCost()

### Community 54 - "Bench — Ring Buffer"
Cohesion: 0.39
Nodes (3): main(), ring_buffer_cost(), ringBufferCost()

### Community 55 - "Bench — CSV Scan"
Cohesion: 0.39
Nodes (3): csv_scan_cost(), csvScanCost(), main()

### Community 56 - "Escape Analysis"
Cohesion: 0.29
Nodes (7): CalleePurityInfo, escape_shim_reports_each_function_without_promotions(), EscapeAnalysisResults, FunctionEscapeSummary, FunctionPromotionSummary, run_escape_and_promote(), StackPromotionResults

### Community 57 - "Verification Cache"
Cohesion: 0.36
Nodes (3): test_verification_cache(), VerificationCache, VerificationStatus

### Community 58 - "Bench — Fibonacci"
Cohesion: 0.48
Nodes (2): fib(), main()

### Community 59 - "Bench — Matrix Multiply"
Cohesion: 0.53
Nodes (2): main(), matrix_checksum()

### Community 60 - "Bench — Tensor MatMul"
Cohesion: 0.53
Nodes (2): main(), matmul_score()

### Community 61 - "Type Expression AST"
Cohesion: 0.33
Nodes (3): TypeExpr, TypeExprKind, TypeMode

### Community 62 - "Runtime Builtins"
Cohesion: 0.4
Nodes (0): 

### Community 63 - "Benchmark Config"
Cohesion: 0.4
Nodes (1): Benchmark infrastructure for the Agam benchmark workspace.

### Community 64 - "Statement AST"
Cohesion: 0.4
Nodes (4): CatchClause, ElseBranch, Stmt, StmtKind

### Community 65 - "Symbol Table"
Cohesion: 0.4
Nodes (4): Symbol, SymbolId, SymbolKind, TypeId

### Community 66 - "Pattern AST"
Cohesion: 0.5
Nodes (3): FieldPattern, Pattern, PatternKind

### Community 67 - "Parser Core"
Cohesion: 0.5
Nodes (1): ParseError

### Community 68 - "MIR Optimizer"
Cohesion: 0.67
Nodes (0): 

### Community 69 - "AST Visitor"
Cohesion: 1.0
Nodes (1): Visitor

### Community 70 - "Tokenizer"
Cohesion: 1.0
Nodes (0): 

### Community 71 - "Codegen"
Cohesion: 1.0
Nodes (0): 

### Community 72 - "Debugger"
Cohesion: 1.0
Nodes (0): 

### Community 73 - "Doc Generator"
Cohesion: 1.0
Nodes (0): 

### Community 74 - "Error Framework"
Cohesion: 1.0
Nodes (0): 

### Community 75 - "Foreign Function Interface"
Cohesion: 1.0
Nodes (0): 

### Community 76 - "Game Engine"
Cohesion: 1.0
Nodes (0): 

### Community 77 - "HIR"
Cohesion: 1.0
Nodes (0): 

### Community 78 - "Linter"
Cohesion: 1.0
Nodes (0): 

### Community 79 - "Macro System"
Cohesion: 1.0
Nodes (0): 

### Community 80 - "MIR"
Cohesion: 1.0
Nodes (0): 

### Community 81 - "Notebooks"
Cohesion: 1.0
Nodes (0): 

### Community 82 - "Runtime"
Cohesion: 1.0
Nodes (0): 

### Community 83 - "Semantic Analysis"
Cohesion: 1.0
Nodes (0): 

### Community 84 - "Formal Verification"
Cohesion: 1.0
Nodes (0): 

### Community 85 - "Standard Library"
Cohesion: 1.0
Nodes (0): 

### Community 86 - "UI Framework"
Cohesion: 1.0
Nodes (0): 

## Knowledge Gaps
- **278 isolated node(s):** `Decl`, `DeclKind`, `Visibility`, `FunctionDecl`, `FunctionParam` (+273 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `AST Visitor`** (2 nodes): `visitor.rs`, `Visitor`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Tokenizer`** (2 nodes): `lib.rs`, `tokenize()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Codegen`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Debugger`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Doc Generator`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Error Framework`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Foreign Function Interface`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Game Engine`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `HIR`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Linter`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Macro System`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `MIR`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Notebooks`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Runtime`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Semantic Analysis`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Formal Verification`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Standard Library`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `UI Framework`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `agam_adam()` connect `Math Primitives` to `Automatic Differentiation`?**
  _High betweenness centrality (0.006) - this node is a cross-community bridge._
- **What connects `Decl`, `DeclKind`, `Visibility` to the rest of the system?**
  _278 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Compiler Driver` be split into smaller, more focused modules?**
  _Cohesion score 0.01 - nodes in this community are weakly interconnected._
- **Should `Package Manager` be split into smaller, more focused modules?**
  _Cohesion score 0.02 - nodes in this community are weakly interconnected._
- **Should `Benchmarks` be split into smaller, more focused modules?**
  _Cohesion score 0.03 - nodes in this community are weakly interconnected._
- **Should `JIT Engine` be split into smaller, more focused modules?**
  _Cohesion score 0.03 - nodes in this community are weakly interconnected._
- **Should `LLVM Codegen` be split into smaller, more focused modules?**
  _Cohesion score 0.04 - nodes in this community are weakly interconnected._