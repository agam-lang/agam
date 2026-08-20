//! Performance and compilation speed benchmarks.

#[cfg(test)]
mod tests {
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_jit::{CompiledJitModule, JitOptions};
    use agam_lexer::tokenize;
    use agam_mir::lower::MirLowering;
    use agam_parser::parse;
    use std::time::Instant;

    #[test]
    fn test_perf_lexer_throughput() {
        let snippet = "let mut x: i32 = 42 + 100 * 3\n";
        let large_src = snippet.repeat(10_000); // 10,000 lines
        let bytes = large_src.len();

        let start = Instant::now();
        let tokens = tokenize(&large_src, SourceId(0));
        let elapsed = start.elapsed();

        assert!(!tokens.is_empty());
        let mb_per_sec = (bytes as f64 / 1_000_000.0) / elapsed.as_secs_f64().max(0.0001);
        // Assert lexer throughput exceeds minimum acceptable baseline (1 MB/s in unoptimized debug build)
        assert!(
            mb_per_sec > 1.0,
            "lexer throughput was {mb_per_sec:.2} MB/s"
        );
    }

    #[test]
    fn test_perf_complete_pipeline_latency() {
        let src = r#"
fn compute_metrics(x: i32, y: i32) -> i32:
    let mut acc: i32 = x
    let mut i: i32 = 0
    while i < 10:
        acc = acc + (y * 2)
        i = i + 1
    return acc

@test
fn test_metrics() -> bool:
    return compute_metrics(5, 3) == 65
"#;
        let source_id = SourceId(0);

        let start = Instant::now();
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mir = mir_lowering.lower_module(&hir);

        let compiled =
            CompiledJitModule::compile(&mir, JitOptions::default()).expect("JIT compile");
        let total_time = start.elapsed();

        assert!(
            total_time.as_millis() < 100,
            "pipeline latency took {:?}",
            total_time
        );
        let res = compiled.run_function("test_metrics", &[]).expect("run");
        assert_eq!(res, agam_jit::JitValue::Bool(true));
    }
}
