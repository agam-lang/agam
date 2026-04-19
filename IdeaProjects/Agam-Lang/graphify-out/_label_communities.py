"""
Generate descriptive community labels from graphify analysis data.

Reads .graphify_analysis.json + .graphify_extract.json, infers a human-readable
label for each community based on its member nodes' names and source files,
then writes a community_labels.json mapping { community_id_str: "Label" }.
"""
import json, re
from pathlib import Path
from collections import Counter

analysis   = json.loads(Path('graphify-out/.graphify_analysis.json').read_text(encoding='utf-8'))
extraction = json.loads(Path('graphify-out/.graphify_extract.json').read_text(encoding='utf-8'))

# Build node metadata lookup: node_id -> { name, source_file, ... }
node_meta = {}
for node in extraction.get('nodes', []):
    nid = node.get('id', '')
    node_meta[nid] = node

communities = analysis['communities']

# ---------------------------------------------------------------------------
# Heuristic labeling rules — ordered by specificity
# ---------------------------------------------------------------------------

# Map: pattern in member names/source files → label
# These are checked in order; first match wins.
KEYWORD_RULES = [
    # Source file path patterns (most reliable)
    (r'agam_driver|main_rs', 'main\.rs', 'CLI Driver & Compiler Pipeline'),
    (r'agam_pkg|lib_rs.*workspace|lib_rs.*lockfile|lib_rs.*registry|lib_rs.*manifest', None, 'Package Management & Registry'),
    (r'benchmark|harness|profiler|statistical_analyzer|result_formatter', None, 'Benchmark Infrastructure'),
    (r'agam_jit|jit', None, 'JIT Compilation'),
    (r'agam_codegen_c|emit_block|compile_to_c|binop_to_c', None, 'C Backend Codegen'),
    (r'parser|parse_expr|parse_src', None, 'Parser'),
    (r'lexer|lex\b|token', None, 'Lexer & Tokenization'),
    (r'cursor', None, 'Cursor (Lexer Utility)'),
    (r'hir_lowering|lower_binop|lower_unaryop|lower_to_mir|lower_source', None, 'HIR/MIR Lowering'),
    (r'ownership|borrow|activeborrow|borrowkind|memorymode', None, 'Ownership & Borrow Checking'),
    (r'type_check|typechecker|check_source|arithmetic_same_type', None, 'Type Checking'),
    (r'type_store|typestore|floatsize|intsize|builtin_type_id', None, 'Type System Core'),
    (r'inference|inferenceengine|unif', None, 'Type Inference & Unification'),
    (r'const_eval|constevaluator|constvalue|fold_binop.*const|fold_unop', None, 'Constant Evaluation'),
    (r'const.*fold|propagat.*constant|fold.*literal', None, 'Constant Folding & Propagation'),
    (r'scope|scopestack|declare_and_lookup|shadowing', None, 'Scope & Name Resolution'),
    (r'resolve.*error|resolver|parse_and_resolve', None, 'Name Resolution'),
    (r'exhaust|exhaustiveness|simplepattern', None, 'Pattern Exhaustiveness Checking'),
    (r'diagnostic_emitter|diag.*emit', None, 'Diagnostic Emission'),
    (r'diagnostic|diagnosticlevel|errorcode|label.*error', None, 'Diagnostics & Error Reporting'),
    (r'smt|z3|smtlib|solver.*div_zero', None, 'SMT Solver / Formal Verification'),
    (r'cache.*artifact|cacheentry|cachehit|cachekey|cachestatus', None, 'Build Cache System'),
    (r'call_cache.*analysis|builtin_call_cache|callcachemode|callcacherequest', None, 'Call Cache Analysis'),
    (r'adaptive_admission|admission_decision', None, 'Adaptive Call Cache Admission'),
    (r'specialization|specialize', None, 'Specialization & Monomorphization'),
    (r'escape_analysis|escape_shim|stackpromotion|calleepurity', None, 'Escape Analysis & Stack Promotion'),
    (r'verification_cache|verificationstatus', None, 'Verification Cache'),
    (r'inline|inlinestate|inline_candidate', None, 'Inlining Optimization'),
    (r'dce|dead.*code|dead_locals|unreachable_blocks', None, 'Dead Code Elimination'),
    (r'loop.*analysis|trip_count|compute_trip|clone_op|find_step', None, 'Loop Analysis & Optimization'),
    (r'effect|cps|handler|effectregistry|effectdef|handlerdef', None, 'Algebraic Effects & CPS'),
    (r'lsp|did_change|did_close|did_open|format_document|hover', None, 'Language Server (LSP)'),
    (r'source_file|sourceid|span\b|line_col', None, 'Source Files & Spans'),
    (r'pretty_print|prettyprinter', None, 'Pretty Printer'),
    (r'format_source|format_inputs|format_path|expand.*tabs', None, 'Code Formatter'),
    (r'simd|axpy|dispatch_binary|alignment.*hint|neon|avx|sse2', None, 'SIMD & Vectorized Operations'),
    (r'hwinfo|hardwareinfo|simdcapabilities|simdtier|cacheinfo|endianness', None, 'Hardware Detection'),
    (r'agam_arc|archeader|arcinner|retain_release', None, 'Reference Counting (AgamArc)'),
    (r'package_load|runtimeabi|host_runtime|portable_manifest', None, 'Package Loading & Runtime ABI'),
    (r'dataframe|column|sample_df|df_filter|df_describe', None, 'DataFrame (std::data)'),
    (r'tensor|test_add.*tensor|test_dot|test_matmul|test_relu', None, 'Tensor (std::tensor)'),
    (r'dual|gradtape|tapenode|autodiff|chain_rule', None, 'Autodiff (std::autodiff)'),
    (r'complex|quaternion', None, 'Complex & Quaternion Math'),
    (r'biguint|rng|stats|welch_t_test|interval', None, 'Numeric Types & Statistics'),
    (r'ndarray|arange|cumsum|concatenate|linspace|diag|eye|flatten', None, 'NDArray Operations (std::ndarray)'),
    (r'linalg|matrix|det|inverse|lu_decompose|matvec|eigenvalue', None, 'Linear Algebra (std::linalg)'),
    (r'batch_norm|cross_entropy|dense|knn|gelu|huber|f1_score|euclidean', None, 'ML Utilities (std::ml)'),
    (r'adam\b|linear_regression|rk4|ode', None, 'Numerical Solvers (std::solver)'),
    (r'bisect|fft|newton|integrate_simpson|integrate_gauss', None, 'Numerical Methods (std::numeric)'),
    (r'mir_function|mirmodule|basicblock|blockid|instruction|mirbinop', None, 'MIR Data Structures'),
    (r'hir_block|hirexpr|hirfunction|hirmodule|hirbinop', None, 'HIR Data Structures'),
    (r'ast.*expr|exprkind|block|binop|fstringpart|lambdaparam|matcharm', None, 'AST Expression Nodes'),
    (r'ast.*stmt|stmtkind|catchclause|elsebranch', None, 'AST Statement Nodes'),
    (r'ast.*type_expr|typeexpr|typemode', None, 'AST Type Expressions'),
    (r'symbol\b|symbolid|symbolkind|typeid\b', None, 'Symbol Table'),
    (r'pattern|patternkind|fieldpattern', None, 'Pattern AST Nodes'),
    (r'parseerror', None, 'Parse Error'),
    (r'visitor', None, 'AST Visitor'),
    (r'annotation|attribute|decl\b|declkind|enumdecl|enumvariant|funcdecl', None, 'AST Declaration Nodes'),
    (r'edit_distance|editdistancecost', None, 'Benchmark: Edit Distance'),
    (r'polynomial|polynomialcost', None, 'Benchmark: Polynomial Eval'),
    (r'ring_buffer|ringbuffercost', None, 'Benchmark: Ring Buffer'),
    (r'csv_scan|csvscanc', None, 'Benchmark: CSV Scanning'),
    (r'fib\b|fibonacci', None, 'Benchmark: Fibonacci'),
    (r'matrix_checksum', None, 'Benchmark: Matrix Checksum'),
    (r'matmul_score', None, 'Benchmark: Matrix Multiply'),
    (r'daemon|prewarm', None, 'Daemon & Prewarming'),
    (r'lifetime', None, 'Lifetime Analysis'),
]

def infer_label(cid_str, member_ids):
    """Given a community ID and its member node IDs, return a descriptive label."""
    if not member_ids:
        return f'Isolate {cid_str}'

    # Collect all member names (cleaned)
    names = []
    source_files = set()
    for mid in member_ids:
        meta = node_meta.get(mid, {})
        name = meta.get('name', mid)
        names.append(name.lower())
        sf = meta.get('source_file', '')
        if sf:
            source_files.add(sf.lower())

    combined = ' '.join(names) + ' ' + ' '.join(source_files)

    for rule in KEYWORD_RULES:
        pattern, extra_pattern, label = rule
        if re.search(pattern, combined, re.IGNORECASE):
            if extra_pattern is None or re.search(extra_pattern, combined, re.IGNORECASE):
                return label

    # Fallback: use the most common crate/module prefix
    prefixes = Counter()
    for mid in member_ids:
        parts = mid.split('_')
        if len(parts) >= 2:
            prefixes[parts[0]] += 1
    if prefixes:
        top = prefixes.most_common(1)[0][0]
        return f'{top.title()} Module'

    return f'Community {cid_str}'

# ---------------------------------------------------------------------------
# Generate labels
# ---------------------------------------------------------------------------
labels = {}
used_labels = Counter()

for cid_str, members in sorted(communities.items(), key=lambda x: int(x[0])):
    label = infer_label(cid_str, members)
    used_labels[label] += 1
    labels[cid_str] = label

# Disambiguate duplicates by appending a number
label_counts = Counter()
final_labels = {}
for cid_str in sorted(labels.keys(), key=int):
    label = labels[cid_str]
    if used_labels[label] > 1:
        label_counts[label] += 1
        suffix = label_counts[label]
        final_labels[cid_str] = f'{label} ({suffix})'
    else:
        final_labels[cid_str] = label

# Write output
out = Path('graphify-out/community_labels.json')
out.write_text(json.dumps(final_labels, indent=2, ensure_ascii=False), encoding='utf-8')

print(f'Generated {len(final_labels)} community labels:')
for cid in sorted(final_labels.keys(), key=int):
    print(f'  {cid:>3}: {final_labels[cid]}')
