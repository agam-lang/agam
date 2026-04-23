//! C code emitter — translates MIR into C source code.
//!
//! The generated C code can be compiled with any standard C compiler
//! to produce native binaries. This is the simplest path to running
//! Agam programs natively without requiring LLVM bindings.

use agam_mir::ir::*;
use agam_sema::types::{FloatSize, Type, builtin_type_by_id};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CType {
    Int,
    Float,
    Bool,
    Str,
    Tensor,
    DataFrame,
}

impl CType {
    fn name(self) -> &'static str {
        match self {
            CType::Int => "agam_int",
            CType::Float => "agam_float",
            CType::Bool => "agam_bool",
            CType::Str => "agam_str",
            CType::Tensor => "AgamTensor*",
            CType::DataFrame => "AgamDataFrame*",
        }
    }

    fn default_value(self) -> &'static str {
        match self {
            CType::Int => "0",
            CType::Float => "0.0",
            CType::Bool => "0",
            CType::Str => "NULL",
            CType::Tensor | CType::DataFrame => "NULL",
        }
    }
}

#[derive(Clone, Copy)]
struct BuiltinSig {
    return_ty: CType,
}

struct FunctionLayout {
    params: Vec<CType>,
    return_ty: CType,
    value_types: HashMap<ValueId, CType>,
    local_types: HashMap<String, CType>,
}

fn analyze_module(module: &MirModule) -> HashMap<String, FunctionLayout> {
    let mut return_types: HashMap<String, CType> = module
        .functions
        .iter()
        .map(|func| {
            (
                func.name.clone(),
                infer_ctype_from_type_id(func.return_ty).unwrap_or(CType::Int),
            )
        })
        .collect();

    let mut layouts = HashMap::new();

    loop {
        let mut changed = false;

        for func in &module.functions {
            let layout = analyze_function(func, &return_types);
            let prev = return_types.insert(func.name.clone(), layout.return_ty);
            changed |= prev != Some(layout.return_ty);
            layouts.insert(func.name.clone(), layout);
        }

        if !changed {
            break;
        }
    }

    layouts
}

fn analyze_function(func: &MirFunction, return_types: &HashMap<String, CType>) -> FunctionLayout {
    let mut layout = FunctionLayout {
        params: func
            .params
            .iter()
            .map(|param| infer_ctype_from_type_id(param.ty).unwrap_or(CType::Int))
            .collect(),
        return_ty: infer_ctype_from_type_id(func.return_ty).unwrap_or(CType::Int),
        value_types: HashMap::new(),
        local_types: HashMap::new(),
    };

    for (index, param) in func.params.iter().enumerate() {
        let ty = layout.params.get(index).copied().unwrap_or(CType::Int);
        layout.value_types.insert(param.value, ty);
        layout.local_types.insert(param.name.clone(), ty);
    }

    for block in &func.blocks {
        for instr in &block.instructions {
            let inferred = match &instr.op {
                Op::ConstInt(_) => CType::Int,
                Op::ConstFloat(_) => CType::Float,
                Op::ConstBool(_) => CType::Bool,
                Op::ConstString(_) => CType::Str,
                Op::Unit => CType::Int,
                Op::Copy(value) => value_type(&layout, *value),
                Op::BinOp { op, left, right } => {
                    infer_binop_type(*op, value_type(&layout, *left), value_type(&layout, *right))
                }
                Op::UnOp { op, operand } => infer_unop_type(*op, value_type(&layout, *operand)),
                Op::Call { callee, .. } => {
                    if is_print_builtin(callee) {
                        CType::Int
                    } else if let Some(sig) = builtin_signature(callee) {
                        sig.return_ty
                    } else {
                        return_types.get(callee).copied().unwrap_or(CType::Int)
                    }
                }
                Op::LoadLocal(name) => layout
                    .local_types
                    .get(name)
                    .copied()
                    .or_else(|| infer_ctype_from_type_id(instr.ty))
                    .unwrap_or(CType::Int),
                Op::StoreLocal { name, value } => {
                    let ty = layout
                        .local_types
                        .get(name)
                        .copied()
                        .or_else(|| infer_ctype_from_type_id(instr.ty))
                        .unwrap_or_else(|| value_type(&layout, *value));
                    layout.local_types.insert(name.clone(), ty);
                    ty
                }
                Op::Alloca { name, ty } => {
                    let ty = infer_ctype_from_type_id(*ty)
                        .or_else(|| layout.local_types.get(name).copied())
                        .unwrap_or(CType::Int);
                    layout.local_types.entry(name.clone()).or_insert(ty);
                    ty
                }
                Op::GetField { object, .. } => value_type(&layout, *object),
                Op::GetIndex { object, .. } => value_type(&layout, *object),
                Op::Phi(entries) => entries
                    .iter()
                    .map(|(_, value)| value_type(&layout, *value))
                    .reduce(merge_type)
                    .unwrap_or(CType::Int),
                Op::Cast { target_ty, value } => infer_ctype_from_type_id(*target_ty)
                    .unwrap_or_else(|| value_type(&layout, *value)),
                Op::EffectPerform {
                    effect, operation, ..
                } => infer_effect_return_type(effect, operation),
                Op::HandleWith { .. } => CType::Int,
                Op::GpuKernelLaunch { .. } => CType::Int,
                Op::GpuIntrinsic { .. } => CType::Int,
                Op::InlineAsm { .. } => CType::Int,
            };

            layout.value_types.insert(instr.result, inferred);
        }
    }

    let mut return_values = Vec::new();
    for block in &func.blocks {
        if let Terminator::Return(value) = &block.terminator {
            return_values.push(value_type(&layout, *value));
        }
    }
    if !return_values.is_empty() {
        layout.return_ty = return_values
            .into_iter()
            .reduce(merge_type)
            .unwrap_or(layout.return_ty);
    }

    layout
}

fn infer_ctype_from_type_id(type_id: agam_sema::symbol::TypeId) -> Option<CType> {
    match builtin_type_by_id(type_id)? {
        Type::Bool => Some(CType::Bool),
        Type::Str => Some(CType::Str),
        Type::Float(FloatSize::F32 | FloatSize::F64) => Some(CType::Float),
        Type::Int(_) | Type::UInt(_) => Some(CType::Int),
        _ => None,
    }
}

fn builtin_signature(name: &str) -> Option<BuiltinSig> {
    match name {
        "argc" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        "argv" => Some(BuiltinSig {
            return_ty: CType::Str,
        }),
        "parse_int" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        "clock" => Some(BuiltinSig {
            return_ty: CType::Float,
        }),
        "adam" => Some(BuiltinSig {
            return_ty: CType::Float,
        }),
        "dataframe_build_sin" => Some(BuiltinSig {
            return_ty: CType::DataFrame,
        }),
        "dataframe_filter_gt" => Some(BuiltinSig {
            return_ty: CType::DataFrame,
        }),
        "dataframe_sort" => Some(BuiltinSig {
            return_ty: CType::DataFrame,
        }),
        "dataframe_group_by" => Some(BuiltinSig {
            return_ty: CType::DataFrame,
        }),
        "dataframe_mean" => Some(BuiltinSig {
            return_ty: CType::Float,
        }),
        "dataframe_free" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        "tensor_fill_rand" => Some(BuiltinSig {
            return_ty: CType::Tensor,
        }),
        "dense_layer" => Some(BuiltinSig {
            return_ty: CType::Tensor,
        }),
        "conv2d" => Some(BuiltinSig {
            return_ty: CType::Tensor,
        }),
        "tensor_checksum" => Some(BuiltinSig {
            return_ty: CType::Float,
        }),
        "tensor_free" => Some(BuiltinSig {
            return_ty: CType::Int,
        }),
        _ => None,
    }
}

fn is_print_builtin(name: &str) -> bool {
    matches!(name, "print" | "println" | "print_int" | "print_str")
}

fn has_runtime_prelude_definition(name: &str) -> bool {
    is_print_builtin(name) || builtin_signature(name).is_some() || is_effect_prelude_function(name)
}

/// Return true if `name` is an effect dispatch function emitted in the prelude.
fn is_effect_prelude_function(name: &str) -> bool {
    name.starts_with("agam_effect_")
}

/// Infer the C return type for a specific effect operation.
fn infer_effect_return_type(effect: &str, operation: &str) -> CType {
    match (effect, operation) {
        // FileSystem returns
        ("FileSystem", "exists" | "is_file" | "is_dir") => CType::Bool,
        ("FileSystem", "read_to_string") => CType::Str,
        ("FileSystem", "read_lines") => CType::Str, // simplified: returns joined string
        ("FileSystem", "list_dir") => CType::Str,   // simplified: returns joined string
        ("FileSystem", "create_dir_all" | "write_string" | "append_string") => CType::Int,
        // Console returns
        ("Console", "read_line") => CType::Str,
        ("Console", "print" | "println" | "eprint" | "eprintln") => CType::Int,
        _ => CType::Int,
    }
}

/// Map an effect+operation pair to its emitted C function name.
fn effect_c_function_name(effect: &str, operation: &str) -> String {
    format!("agam_effect_{}_{}", effect, operation)
}

fn infer_binop_type(op: MirBinOp, left: CType, right: CType) -> CType {
    match op {
        MirBinOp::Eq
        | MirBinOp::NotEq
        | MirBinOp::Lt
        | MirBinOp::LtEq
        | MirBinOp::Gt
        | MirBinOp::GtEq
        | MirBinOp::And
        | MirBinOp::Or => CType::Bool,
        MirBinOp::Add if left == CType::Str || right == CType::Str => CType::Str,
        _ if left == CType::DataFrame || right == CType::DataFrame => CType::DataFrame,
        _ if left == CType::Tensor || right == CType::Tensor => CType::Tensor,
        _ if left == CType::Float || right == CType::Float => CType::Float,
        _ => CType::Int,
    }
}

fn infer_unop_type(op: MirUnOp, operand: CType) -> CType {
    match op {
        MirUnOp::Not => CType::Bool,
        MirUnOp::Neg if operand == CType::Float => CType::Float,
        _ => CType::Int,
    }
}

fn merge_type(left: CType, right: CType) -> CType {
    if left == right {
        left
    } else if left == CType::DataFrame || right == CType::DataFrame {
        CType::DataFrame
    } else if left == CType::Tensor || right == CType::Tensor {
        CType::Tensor
    } else if left == CType::Str || right == CType::Str {
        CType::Str
    } else if left == CType::Float || right == CType::Float {
        CType::Float
    } else if left == CType::Bool || right == CType::Bool {
        CType::Bool
    } else {
        CType::Int
    }
}

fn value_type(layout: &FunctionLayout, value: ValueId) -> CType {
    layout
        .value_types
        .get(&value)
        .copied()
        .unwrap_or(CType::Int)
}

fn emit_common_prelude(out: &mut String) {
    out.push_str(
        r#"/* ── Agam Runtime Prelude ──────────────────── */
agam_int agam_println(agam_str s) { printf("%s\n", s); return 0; }
agam_int agam_print(agam_str s) { printf("%s", s); return 0; }
agam_float agam_clock(void) { return (agam_float)clock() / (agam_float)CLOCKS_PER_SEC; }
static agam_int agam_runtime_argc = 0;
static char** agam_runtime_argv = NULL;
agam_int agam_argc(void) { return agam_runtime_argc; }
agam_str agam_argv(agam_int index) {
  if (!agam_runtime_argv || index < 0 || index >= agam_runtime_argc) {
    return "";
  }
  return agam_runtime_argv[index];
}
agam_int agam_parse_int(agam_str s) {
  if (!s) {
    return 0;
  }
  return (agam_int)strtoll(s, NULL, 10);
}

agam_str agam_str_concat(agam_str a, agam_str b) {
  size_t a_len = strlen(a);
  size_t b_len = strlen(b);
  char* out = (char*)malloc(a_len + b_len + 1);
  memcpy(out, a, a_len);
  memcpy(out + a_len, b, b_len + 1);
  return out;
}

typedef struct AgamTensor {
  agam_int rows;
  agam_int cols;
  agam_int len;
  agam_float* data;
} AgamTensor;

typedef struct AgamDataFrame {
  agam_int len;
  agam_int* ids;
  agam_int* groups;
  agam_float* scores;
} AgamDataFrame;

typedef struct AgamRow {
  agam_int id;
  agam_int group;
  agam_float score;
} AgamRow;

static uint64_t agam_mix64(uint64_t x) {
  x ^= x >> 33;
  x *= 0xff51afd7ed558ccdULL;
  x ^= x >> 33;
  x *= 0xc4ceb9fe1a85ec53ULL;
  x ^= x >> 33;
  return x;
}

static uint64_t agam_seed_bits(agam_float seed) {
  union {
    agam_float f;
    uint64_t u;
  } bits;
  bits.f = seed;
  return bits.u;
}

static agam_float agam_hash_unit(agam_int index, agam_float seed, agam_int salt) {
  uint64_t mixed = agam_mix64((uint64_t)index ^ agam_seed_bits(seed) ^ ((uint64_t)salt << 32));
  return (agam_float)((mixed >> 11) * (1.0 / 9007199254740992.0));
}

static agam_float agam_weight_sample(agam_int row, agam_int col, agam_float seed) {
  return agam_hash_unit(row * 4099 + col * 131, seed, 17) * 2.0 - 1.0;
}

static agam_float agam_bias_sample(agam_int index, agam_float seed) {
  return agam_hash_unit(index * 7919, seed, 29) * 0.25 - 0.125;
}

static AgamTensor* agam_tensor_new(agam_int rows, agam_int cols) {
  AgamTensor* tensor = (AgamTensor*)malloc(sizeof(AgamTensor));
  tensor->rows = rows;
  tensor->cols = cols;
  tensor->len = rows * cols;
  tensor->data = tensor->len > 0 ? (agam_float*)malloc(sizeof(agam_float) * (size_t)tensor->len) : NULL;
  return tensor;
}

static AgamDataFrame* agam_dataframe_new(agam_int len) {
  AgamDataFrame* df = (AgamDataFrame*)malloc(sizeof(AgamDataFrame));
  df->len = len;
  df->ids = len > 0 ? (agam_int*)malloc(sizeof(agam_int) * (size_t)len) : NULL;
  df->groups = len > 0 ? (agam_int*)malloc(sizeof(agam_int) * (size_t)len) : NULL;
  df->scores = len > 0 ? (agam_float*)malloc(sizeof(agam_float) * (size_t)len) : NULL;
  return df;
}

static int agam_compare_rows_by_score_desc(const void* left, const void* right) {
  const AgamRow* a = (const AgamRow*)left;
  const AgamRow* b = (const AgamRow*)right;
  if (a->score < b->score) {
    return 1;
  }
  if (a->score > b->score) {
    return -1;
  }
  return 0;
}

"#,
    );
}

fn emit_dataframe_prelude(out: &mut String) {
    out.push_str(
        r#"agam_float agam_adam(agam_float x0, agam_float y0, agam_float learning_rate, agam_int max_iter, agam_float tol) {
  agam_float x = x0;
  agam_float y = y0;
  agam_float mx = 0.0;
  agam_float my = 0.0;
  agam_float vx = 0.0;
  agam_float vy = 0.0;
  const agam_float beta1 = 0.9;
  const agam_float beta2 = 0.999;
  const agam_float epsilon = 1e-8;

  for (agam_int t = 1; t <= max_iter; ++t) {
    agam_float dx = -2.0 * (1.0 - x) - 400.0 * x * (y - x * x);
    agam_float dy = 200.0 * (y - x * x);
    agam_float grad_norm = sqrt(dx * dx + dy * dy);
    if (grad_norm < tol) {
      break;
    }

    mx = beta1 * mx + (1.0 - beta1) * dx;
    my = beta1 * my + (1.0 - beta1) * dy;
    vx = beta2 * vx + (1.0 - beta2) * dx * dx;
    vy = beta2 * vy + (1.0 - beta2) * dy * dy;

    agam_float t_f = (agam_float)t;
    agam_float mx_hat = mx / (1.0 - pow(beta1, t_f));
    agam_float my_hat = my / (1.0 - pow(beta1, t_f));
    agam_float vx_hat = vx / (1.0 - pow(beta2, t_f));
    agam_float vy_hat = vy / (1.0 - pow(beta2, t_f));

    x -= learning_rate * mx_hat / (sqrt(vx_hat) + epsilon);
    y -= learning_rate * my_hat / (sqrt(vy_hat) + epsilon);
  }

  {
    agam_float a = 1.0 - x;
    agam_float b = y - x * x;
    return a * a + 100.0 * b * b;
  }
}

AgamDataFrame* agam_dataframe_build_sin(agam_int rows) {
  AgamDataFrame* df = agam_dataframe_new(rows);
  for (agam_int i = 0; i < rows; ++i) {
    df->ids[i] = i;
    df->groups[i] = i % 1024;
    df->scores[i] = sin((agam_float)i * 0.1);
  }
  return df;
}

AgamDataFrame* agam_dataframe_filter_gt(AgamDataFrame* df, agam_float threshold) {
  if (!df) {
    return NULL;
  }

  agam_int count = 0;
  for (agam_int i = 0; i < df->len; ++i) {
    if (df->scores[i] > threshold) {
      ++count;
    }
  }

  AgamDataFrame* filtered = agam_dataframe_new(count);
  agam_int out_index = 0;
  for (agam_int i = 0; i < df->len; ++i) {
    if (df->scores[i] > threshold) {
      filtered->ids[out_index] = df->ids[i];
      filtered->groups[out_index] = df->groups[i];
      filtered->scores[out_index] = df->scores[i];
      ++out_index;
    }
  }
  return filtered;
}

AgamDataFrame* agam_dataframe_sort(AgamDataFrame* df) {
  if (!df) {
    return NULL;
  }

  AgamRow* rows = df->len > 0 ? (AgamRow*)malloc(sizeof(AgamRow) * (size_t)df->len) : NULL;
  for (agam_int i = 0; i < df->len; ++i) {
    rows[i].id = df->ids[i];
    rows[i].group = df->groups[i];
    rows[i].score = df->scores[i];
  }

  qsort(rows, (size_t)df->len, sizeof(AgamRow), agam_compare_rows_by_score_desc);

  AgamDataFrame* sorted = agam_dataframe_new(df->len);
  for (agam_int i = 0; i < df->len; ++i) {
    sorted->ids[i] = rows[i].id;
    sorted->groups[i] = rows[i].group;
    sorted->scores[i] = rows[i].score;
  }

  free(rows);
  return sorted;
}

AgamDataFrame* agam_dataframe_group_by(AgamDataFrame* df, agam_int group_count) {
  if (!df) {
    return NULL;
  }
  if (group_count <= 0) {
    group_count = 1;
  }

  agam_float* sums = (agam_float*)calloc((size_t)group_count, sizeof(agam_float));
  agam_int* counts = (agam_int*)calloc((size_t)group_count, sizeof(agam_int));

  for (agam_int i = 0; i < df->len; ++i) {
    agam_int bucket = df->groups[i] % group_count;
    if (bucket < 0) {
      bucket += group_count;
    }
    sums[bucket] += df->scores[i];
    counts[bucket] += 1;
  }

  agam_int used = 0;
  for (agam_int i = 0; i < group_count; ++i) {
    if (counts[i] > 0) {
      ++used;
    }
  }

  AgamDataFrame* grouped = agam_dataframe_new(used);
  agam_int out_index = 0;
  for (agam_int i = 0; i < group_count; ++i) {
    if (counts[i] > 0) {
      grouped->ids[out_index] = i;
      grouped->groups[out_index] = i;
      grouped->scores[out_index] = sums[i] / (agam_float)counts[i];
      ++out_index;
    }
  }

  free(sums);
  free(counts);
  return grouped;
}

agam_float agam_dataframe_mean(AgamDataFrame* df) {
  if (!df || df->len == 0) {
    return 0.0;
  }

  agam_float sum = 0.0;
  for (agam_int i = 0; i < df->len; ++i) {
    sum += df->scores[i];
  }
  return sum / (agam_float)df->len;
}

agam_int agam_dataframe_free(AgamDataFrame* df) {
  if (!df) {
    return 0;
  }
  free(df->ids);
  free(df->groups);
  free(df->scores);
  free(df);
  return 0;
}

"#,
    );
}

fn emit_tensor_prelude(out: &mut String) {
    out.push_str(
        r#"AgamTensor* agam_tensor_fill_rand(agam_int rows, agam_int cols, agam_float seed) {
  AgamTensor* tensor = agam_tensor_new(rows, cols);
  for (agam_int i = 0; i < tensor->len; ++i) {
    tensor->data[i] = agam_hash_unit(i, seed, 43) * 2.0 - 1.0;
  }
  return tensor;
}

AgamTensor* agam_dense_layer(AgamTensor* input, agam_int out_features, agam_float seed) {
  if (!input || out_features <= 0) {
    return NULL;
  }

  AgamTensor* output = agam_tensor_new(input->rows, out_features);
  for (agam_int row = 0; row < input->rows; ++row) {
    for (agam_int col = 0; col < out_features; ++col) {
      agam_float acc = agam_bias_sample(col, seed);
      for (agam_int inner = 0; inner < input->cols; ++inner) {
        agam_float weight = agam_weight_sample(inner, col, seed);
        acc += input->data[row * input->cols + inner] * weight;
      }
      output->data[row * out_features + col] = acc > 0.0 ? acc : 0.0;
    }
  }
  return output;
}

AgamTensor* agam_conv2d(AgamTensor* input, agam_int kernel_size, agam_float seed) {
  if (!input || kernel_size <= 0 || input->rows < kernel_size || input->cols < kernel_size) {
    return NULL;
  }

  agam_int out_rows = input->rows - kernel_size + 1;
  agam_int out_cols = input->cols - kernel_size + 1;
  AgamTensor* output = agam_tensor_new(out_rows, out_cols);

  for (agam_int y = 0; y < out_rows; ++y) {
    for (agam_int x = 0; x < out_cols; ++x) {
      agam_float acc = 0.0;
      for (agam_int ky = 0; ky < kernel_size; ++ky) {
        for (agam_int kx = 0; kx < kernel_size; ++kx) {
          agam_float kernel = agam_weight_sample(ky, kx, seed);
          agam_float value = input->data[(y + ky) * input->cols + (x + kx)];
          acc += value * kernel;
        }
      }
      output->data[y * out_cols + x] = acc;
    }
  }
  return output;
}

agam_float agam_tensor_checksum(AgamTensor* tensor) {
  if (!tensor || tensor->len == 0) {
    return 0.0;
  }

  agam_float sum = 0.0;
  for (agam_int i = 0; i < tensor->len; ++i) {
    sum += tensor->data[i] * (1.0 + (agam_float)(i & 7));
  }
  return sum;
}

agam_int agam_tensor_free(AgamTensor* tensor) {
  if (!tensor) {
    return 0;
  }
  free(tensor->data);
  free(tensor);
  return 0;
}

"#,
    );
}

fn emit_effect_prelude(out: &mut String) {
    out.push_str(
        r#"/* ── Agam Effect Runtime ───────────────────── */
#include <sys/stat.h>
#include <errno.h>

/* FileSystem.exists(path) -> bool */
agam_bool agam_effect_FileSystem_exists(agam_str path) {
  struct stat st;
  return stat(path, &st) == 0 ? 1 : 0;
}

/* FileSystem.is_file(path) -> bool */
agam_bool agam_effect_FileSystem_is_file(agam_str path) {
  struct stat st;
  if (stat(path, &st) != 0) return 0;
  return S_ISREG(st.st_mode) ? 1 : 0;
}

/* FileSystem.is_dir(path) -> bool */
agam_bool agam_effect_FileSystem_is_dir(agam_str path) {
  struct stat st;
  if (stat(path, &st) != 0) return 0;
  return S_ISDIR(st.st_mode) ? 1 : 0;
}

/* FileSystem.create_dir_all(path) */
agam_int agam_effect_FileSystem_create_dir_all(agam_str path) {
  /* Simple recursive mkdir for POSIX; on Windows use _mkdir */
#ifdef _WIN32
  (void)_mkdir(path);
#else
  mkdir(path, 0755);
#endif
  return 0;
}

/* FileSystem.read_to_string(path) -> string */
agam_str agam_effect_FileSystem_read_to_string(agam_str path) {
  FILE* f = fopen(path, "rb");
  if (!f) return "";
  fseek(f, 0, SEEK_END);
  long len = ftell(f);
  fseek(f, 0, SEEK_SET);
  char* buf = (char*)malloc((size_t)len + 1);
  if (len > 0) {
    size_t read = fread(buf, 1, (size_t)len, f);
    buf[read] = '\0';
  } else {
    buf[0] = '\0';
  }
  fclose(f);
  return buf;
}

/* FileSystem.read_lines(path) -> string (newline-joined) */
agam_str agam_effect_FileSystem_read_lines(agam_str path) {
  return agam_effect_FileSystem_read_to_string(path);
}

/* FileSystem.write_string(path, contents) */
agam_int agam_effect_FileSystem_write_string(agam_str path, agam_str contents) {
  FILE* f = fopen(path, "w");
  if (!f) return -1;
  fputs(contents, f);
  fclose(f);
  return 0;
}

/* FileSystem.append_string(path, contents) */
agam_int agam_effect_FileSystem_append_string(agam_str path, agam_str contents) {
  FILE* f = fopen(path, "a");
  if (!f) return -1;
  fputs(contents, f);
  fclose(f);
  return 0;
}

/* FileSystem.list_dir(path) -> string (newline-joined entries) */
agam_str agam_effect_FileSystem_list_dir(agam_str path) {
  (void)path;
  return ""; /* simplified: full implementation requires dirent.h */
}

/* Console.print(msg) */
agam_int agam_effect_Console_print(agam_str msg) {
  printf("%s", msg);
  return 0;
}

/* Console.println(msg) */
agam_int agam_effect_Console_println(agam_str msg) {
  printf("%s\n", msg);
  return 0;
}

/* Console.read_line() -> string */
agam_str agam_effect_Console_read_line(void) {
  char* buf = (char*)malloc(4096);
  if (!fgets(buf, 4096, stdin)) {
    buf[0] = '\0';
  }
  /* Strip trailing newline */
  size_t len = strlen(buf);
  if (len > 0 && buf[len-1] == '\n') buf[len-1] = '\0';
  return buf;
}

/* Console.eprint(msg) */
agam_int agam_effect_Console_eprint(agam_str msg) {
  fprintf(stderr, "%s", msg);
  return 0;
}

/* Console.eprintln(msg) */
agam_int agam_effect_Console_eprintln(agam_str msg) {
  fprintf(stderr, "%s\n", msg);
  return 0;
}

"#,
    );
}

fn emit_runtime_prelude(out: &mut String, module: &MirModule) {
    use agam_sema::target::TargetProfile;

    // Determine if any function targets IoT (most restrictive)
    let has_iot = module
        .functions
        .iter()
        .any(|f| f.target == TargetProfile::Iot);
    let has_hpc = module
        .functions
        .iter()
        .any(|f| f.target == TargetProfile::Hpc);

    emit_common_prelude(out);

    if has_hpc {
        out.push_str("/* ── HPC Target: aggressive vectorization preferred ── */\n");
        out.push_str("#ifndef AGAM_HPC\n#define AGAM_HPC 1\n#endif\n\n");
    }

    if has_iot {
        out.push_str("/* ── IoT Target: no heap, no stdlib, no effects ── */\n");
        out.push_str("#ifndef AGAM_NO_HEAP\n#define AGAM_NO_HEAP 1\n#endif\n\n");
    } else {
        // Only emit heavy preludes when not targeting IoT
        emit_dataframe_prelude(out);
        emit_tensor_prelude(out);
        emit_effect_prelude(out);
    }
    out.push('\n');
}

/// Emit a complete MIR module as C source code.
pub fn emit_c(module: &MirModule) -> String {
    let layouts = analyze_module(module);
    let mut output = String::new();

    // Header
    writeln!(output, "/* Generated by agamc — Agam Compiler */").unwrap();
    writeln!(output, "#include <stdio.h>").unwrap();
    writeln!(output, "#include <stdlib.h>").unwrap();
    writeln!(output, "#include <string.h>").unwrap();
    writeln!(output, "#include <stdint.h>").unwrap();
    writeln!(output, "#include <math.h>").unwrap();
    writeln!(output, "#include <time.h>").unwrap();
    writeln!(output).unwrap();

    // Type aliases
    writeln!(output, "typedef int64_t agam_int;").unwrap();
    writeln!(output, "typedef double agam_float;").unwrap();
    writeln!(output, "typedef int agam_bool;").unwrap();
    writeln!(output, "typedef const char* agam_str;").unwrap();
    writeln!(output).unwrap();

    emit_runtime_prelude(&mut output, module);

    // Collect all unknown function calls and generate stub declarations
    let mut unknown_funcs: HashSet<String> = HashSet::new();
    for func in &module.functions {
        let layout = layouts.get(&func.name).expect("missing function layout");
        for block in &func.blocks {
            for instr in &block.instructions {
                if let Op::Call { callee, args } = &instr.op {
                    let mangled = mangle_name(callee);
                    if !has_runtime_prelude_definition(callee)
                        && !module
                            .functions
                            .iter()
                            .any(|f| mangle_name(&f.name) == mangled)
                    {
                        let ret_ty = value_type(layout, instr.result).name();
                        unknown_funcs.insert(format!(
                            "{} {}({});",
                            ret_ty,
                            mangled,
                            args.iter()
                                .enumerate()
                                .map(|(i, arg)| format!(
                                    "{} __a{}",
                                    value_type(layout, *arg).name(),
                                    i
                                ))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                }
            }
        }
    }
    if !unknown_funcs.is_empty() {
        writeln!(output, "/* ── External function stubs ── */").unwrap();
        for stub in &unknown_funcs {
            writeln!(output, "{}", stub).unwrap();
        }
        writeln!(output).unwrap();
    }

    // Forward declarations
    for func in &module.functions {
        let layout = layouts.get(&func.name).expect("missing function layout");
        if func.name == "main" {
            writeln!(output, "int main(int argc, char** argv);").unwrap();
        } else {
            write!(
                output,
                "{} {}(",
                layout.return_ty.name(),
                mangle_name(&func.name)
            )
            .unwrap();
            for (i, _) in func.params.iter().enumerate() {
                if i > 0 {
                    write!(output, ", ").unwrap();
                }
                let param_ty = layout.params.get(i).copied().unwrap_or(CType::Int);
                write!(output, "{} __p{}", param_ty.name(), i).unwrap();
            }
            writeln!(output, ");").unwrap();
        }
    }
    writeln!(output).unwrap();

    // Function definitions
    for func in &module.functions {
        let layout = layouts.get(&func.name).expect("missing function layout");
        emit_function(&mut output, func, layout);
        writeln!(output).unwrap();
    }

    output
}

fn emit_function(out: &mut String, func: &MirFunction, layout: &FunctionLayout) {
    // Function signature
    if func.name == "main" {
        writeln!(out, "int main(int argc, char** argv) {{").unwrap();
    } else {
        write!(
            out,
            "{} {}(",
            layout.return_ty.name(),
            mangle_name(&func.name)
        )
        .unwrap();
        for (i, _param) in func.params.iter().enumerate() {
            if i > 0 {
                write!(out, ", ").unwrap();
            }
            let param_ty = layout.params.get(i).copied().unwrap_or(CType::Int);
            write!(out, "{} __p{}", param_ty.name(), i).unwrap();
        }
        writeln!(out, ") {{").unwrap();
    }

    // Emit parameter → local aliases
    if func.name == "main" {
        writeln!(out, "  agam_runtime_argc = (agam_int)argc;").unwrap();
        writeln!(out, "  agam_runtime_argv = argv;").unwrap();
    }
    for (i, param) in func.params.iter().enumerate() {
        let param_ty = layout.params.get(i).copied().unwrap_or(CType::Int);
        writeln!(
            out,
            "  {} {} = __p{};",
            param_ty.name(),
            mangle_local(&param.name),
            i
        )
        .unwrap();
    }

    // Emit all basic blocks
    for block in &func.blocks {
        emit_block(out, block, layout, func.name == "main");
    }

    writeln!(out, "}}").unwrap();
}

fn emit_block(out: &mut String, block: &BasicBlock, layout: &FunctionLayout, is_main: bool) {
    writeln!(out, "block_{}:", block.id.0).unwrap();

    for instr in &block.instructions {
        emit_instruction(out, instr, layout);
    }

    emit_terminator(out, &block.terminator, layout, is_main);
}

fn emit_instruction(out: &mut String, instr: &Instruction, layout: &FunctionLayout) {
    let v = format!("__v{}", instr.result.0);
    let result_ty = value_type(layout, instr.result);

    match &instr.op {
        Op::ConstInt(val) => {
            writeln!(out, "  {} {} = {};", result_ty.name(), v, val).unwrap();
        }
        Op::ConstFloat(val) => {
            writeln!(out, "  {} {} = {};", result_ty.name(), v, val).unwrap();
        }
        Op::ConstBool(val) => {
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                if *val { 1 } else { 0 }
            )
            .unwrap();
        }
        Op::ConstString(val) => {
            writeln!(
                out,
                "  {} {} = \"{}\";",
                result_ty.name(),
                v,
                escape_c_string(val)
            )
            .unwrap();
        }
        Op::Unit => {
            writeln!(
                out,
                "  {} {} = {}; /* unit */",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
        }
        Op::BinOp { op, left, right } => {
            if *op == MirBinOp::Add && result_ty == CType::Str {
                writeln!(
                    out,
                    "  {} {} = agam_str_concat((agam_str)__v{}, (agam_str)__v{});",
                    result_ty.name(),
                    v,
                    left.0,
                    right.0
                )
                .unwrap();
            } else {
                let op_str = binop_to_c(*op);
                writeln!(
                    out,
                    "  {} {} = __v{} {} __v{};",
                    result_ty.name(),
                    v,
                    left.0,
                    op_str,
                    right.0
                )
                .unwrap();
            }
        }
        Op::UnOp { op, operand } => {
            let op_str = unop_to_c(*op);
            writeln!(
                out,
                "  {} {} = {}__v{};",
                result_ty.name(),
                v,
                op_str,
                operand.0
            )
            .unwrap();
        }
        Op::Call { callee, args } => {
            if is_print_builtin(callee) {
                if args.is_empty() {
                    writeln!(out, "  printf(\"\\n\");").unwrap();
                } else {
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            writeln!(out, "  printf(\" \");").unwrap();
                        }
                        emit_print_value(out, *arg, value_type(layout, *arg));
                    }
                    if callee == "println" || callee == "print_str" {
                        writeln!(out, "  printf(\"\\n\");").unwrap();
                    } else if callee == "print" || callee == "print_int" {
                        writeln!(out, "  printf(\"\\n\");").unwrap(); // always append newline for benchmarking clarify
                    }
                }
                writeln!(out, "  {} {} = 0;", result_ty.name(), v).unwrap();
            } else {
                let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
                writeln!(
                    out,
                    "  {} {} = {}({});",
                    result_ty.name(),
                    v,
                    mangle_name(callee),
                    arg_strs.join(", ")
                )
                .unwrap();
            }
        }
        Op::Copy(value) => {
            writeln!(out, "  {} {} = __v{};", result_ty.name(), v, value.0).unwrap();
        }
        Op::LoadLocal(name) => {
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                mangle_local(name)
            )
            .unwrap();
        }
        Op::StoreLocal { name, value } => {
            writeln!(out, "  {} = __v{};", mangle_local(name), value.0).unwrap();
            writeln!(out, "  {} {} = __v{};", result_ty.name(), v, value.0).unwrap();
        }
        Op::Alloca { name, .. } => {
            let local_ty = layout.local_types.get(name).copied().unwrap_or(CType::Int);
            writeln!(
                out,
                "  {} {} = {};",
                local_ty.name(),
                mangle_local(name),
                local_ty.default_value()
            )
            .unwrap();
            writeln!(
                out,
                "  {} {} = {};",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
        }
        Op::GetField { object, field } => {
            writeln!(
                out,
                "  {} {} = __v{}; /* .{} */",
                result_ty.name(),
                v,
                object.0,
                field
            )
            .unwrap();
        }
        Op::GetIndex { object, index } => {
            writeln!(
                out,
                "  {} {} = __v{}; /* [__v{}] */",
                result_ty.name(),
                v,
                object.0,
                index.0
            )
            .unwrap();
        }
        Op::Phi(entries) => {
            writeln!(
                out,
                "  {} {} = {}; /* phi */",
                result_ty.name(),
                v,
                result_ty.default_value()
            )
            .unwrap();
            for (block, val) in entries {
                writeln!(out, "  /* phi: block_{} -> __v{} */", block.0, val.0).unwrap();
            }
        }
        Op::Cast { value, .. } => {
            writeln!(
                out,
                "  {} {} = ({})__v{};",
                result_ty.name(),
                v,
                result_ty.name(),
                value.0
            )
            .unwrap();
        }
        Op::EffectPerform {
            effect,
            operation,
            args,
        } => {
            let func_name = effect_c_function_name(effect, operation);
            let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
            writeln!(
                out,
                "  {} {} = {}({});",
                result_ty.name(),
                v,
                func_name,
                arg_strs.join(", ")
            )
            .unwrap();
        }
        Op::HandleWith {
            effect,
            handler,
            body,
        } => {
            writeln!(
                out,
                "  /* handle {} with {} — executing body block_{} */",
                effect, handler, body.0
            )
            .unwrap();
            writeln!(out, "  {} {} = 0;", result_ty.name(), v).unwrap();
        }
        Op::GpuKernelLaunch {
            kernel_name,
            grid,
            block,
            args,
        } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
            writeln!(
                out,
                "  /* GPU kernel launch: {}<<<__v{}, __v{}>>>({}) */",
                kernel_name,
                grid.0,
                block.0,
                arg_strs.join(", ")
            )
            .unwrap();
            writeln!(out, "  {} {} = 0; /* kernel launch result */", result_ty.name(), v).unwrap();
        }
        Op::GpuIntrinsic { kind, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
            writeln!(
                out,
                "  {} {} = 0; /* GPU intrinsic {:?}({}) */",
                result_ty.name(),
                v,
                kind,
                arg_strs.join(", ")
            )
            .unwrap();
        }
        Op::InlineAsm {
            asm_string,
            constraints,
            args,
        } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format!("__v{}", a.0)).collect();
            writeln!(
                out,
                "  /* inline asm: \"{}\" constraints=\"{}\" args=({}) */",
                asm_string,
                constraints,
                arg_strs.join(", ")
            )
            .unwrap();
            writeln!(out, "  {} {} = 0; /* asm result */", result_ty.name(), v).unwrap();
        }
    }
}

fn emit_terminator(out: &mut String, term: &Terminator, _layout: &FunctionLayout, is_main: bool) {
    match term {
        Terminator::Return(val) => {
            if is_main {
                writeln!(out, "  return (int)__v{};", val.0).unwrap();
            } else {
                writeln!(out, "  return __v{};", val.0).unwrap();
            }
        }
        Terminator::ReturnVoid => {
            writeln!(out, "  return 0;").unwrap();
        }
        Terminator::Jump(block) => {
            writeln!(out, "  goto block_{};", block.0).unwrap();
        }
        Terminator::Branch {
            condition,
            then_block,
            else_block,
        } => {
            writeln!(
                out,
                "  if (__v{}) goto block_{}; else goto block_{};",
                condition.0, then_block.0, else_block.0
            )
            .unwrap();
        }
        Terminator::Unreachable => {
            writeln!(out, "  __builtin_unreachable();").unwrap();
        }
    }
}

fn emit_print_value(out: &mut String, value: ValueId, ty: CType) {
    match ty {
        CType::Str => {
            writeln!(out, "  printf(\"%s\", (agam_str)__v{});", value.0).unwrap();
        }
        CType::Float => {
            writeln!(out, "  printf(\"%.17g\", (double)__v{});", value.0).unwrap();
        }
        CType::Bool => {
            writeln!(
                out,
                "  printf(\"%s\", __v{} ? \"true\" : \"false\");",
                value.0
            )
            .unwrap();
        }
        CType::Int => {
            writeln!(out, "  printf(\"%lld\", (long long)__v{});", value.0).unwrap();
        }
        CType::Tensor => {
            writeln!(out, "  printf(\"<tensor:%p>\", (void*)__v{});", value.0).unwrap();
        }
        CType::DataFrame => {
            writeln!(out, "  printf(\"<dataframe:%p>\", (void*)__v{});", value.0).unwrap();
        }
    }
}

fn binop_to_c(op: MirBinOp) -> &'static str {
    match op {
        MirBinOp::Add => "+",
        MirBinOp::Sub => "-",
        MirBinOp::Mul => "*",
        MirBinOp::Div => "/",
        MirBinOp::Mod => "%",
        MirBinOp::Eq => "==",
        MirBinOp::NotEq => "!=",
        MirBinOp::Lt => "<",
        MirBinOp::LtEq => "<=",
        MirBinOp::Gt => ">",
        MirBinOp::GtEq => ">=",
        MirBinOp::And => "&&",
        MirBinOp::Or => "||",
        MirBinOp::BitAnd => "&",
        MirBinOp::BitOr => "|",
        MirBinOp::BitXor => "^",
        MirBinOp::Shl => "<<",
        MirBinOp::Shr => ">>",
    }
}

fn unop_to_c(op: MirUnOp) -> &'static str {
    match op {
        MirUnOp::Neg => "-",
        MirUnOp::Not => "!",
        MirUnOp::BitNot => "~",
    }
}

fn mangle_name(name: &str) -> String {
    format!("agam_{}", name.replace("::", "_").replace(".", "_"))
}

fn mangle_local(name: &str) -> String {
    format!("_local_{}", name.replace("::", "_").replace(".", "_"))
}

fn escape_c_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;
    use agam_mir::lower::MirLowering;

    fn compile_to_c(source: &str) -> String {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(source, source_id);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == agam_lexer::TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = agam_parser::Parser::new(tokens);
        let module = parser.parse_module(source_id).expect("parse failed");

        let mut hir_lower = HirLowering::new();
        let hir = hir_lower.lower_module(&module);

        let mut mir_lower = MirLowering::new();
        let mut mir = mir_lower.lower_module(&hir);
        agam_mir::opt::optimize_module(&mut mir);

        emit_c(&mir)
    }

    #[test]
    fn test_emit_main_function() {
        let c = compile_to_c("fn main(): return 42");
        assert!(
            c.contains("int main("),
            "C output should have main function"
        );
        assert!(c.contains("42"), "C output should have the literal 42");
        assert!(c.contains("return"), "C output should have return");
    }

    #[test]
    fn test_emit_includes() {
        let c = compile_to_c("fn main(): return 0");
        assert!(c.contains("#include <stdio.h>"));
        assert!(c.contains("#include <stdint.h>"));
    }

    #[test]
    fn test_emit_binary_op() {
        let c = compile_to_c("fn main(): let x = 1 + 2");
        assert!(c.contains("+"), "C output should contain addition");
    }

    #[test]
    fn test_emit_print_call() {
        let c = compile_to_c("fn main(): print(42)");
        assert!(
            c.contains("printf"),
            "C output should emit printf for print()"
        );
    }

    #[test]
    fn test_emit_type_aliases() {
        let c = compile_to_c("fn main(): return 0");
        assert!(c.contains("typedef int64_t agam_int;"));
        assert!(c.contains("typedef double agam_float;"));
    }

    #[test]
    fn test_emit_string_local_type() {
        let c = compile_to_c("fn main() { let name = \"World\"; print(name); }");
        assert!(c.contains("agam_str __v"));
        assert!(c.contains("printf(\"%s\""));
    }

    #[test]
    fn test_emit_float_type() {
        let c = compile_to_c("fn main() { let x = 1.5 + 2.5; }");
        assert!(c.contains("agam_float"));
    }

    #[test]
    fn test_non_main_returns_do_not_truncate_to_int() {
        let c =
            compile_to_c("fn add(a: i32) -> i32 { return a + 1; } fn main() { return add(41); }");
        assert!(c.contains("agam_add("));
        assert!(c.contains("return __v"));
    }

    #[test]
    fn test_ml_runtime_handles_emit_without_external_stubs() {
        let c = compile_to_c(
            "fn main() { \
                let df = dataframe_build_sin(32); \
                let filtered = dataframe_filter_gt(df, 0.5); \
                let grouped = dataframe_group_by(filtered, 8); \
                let mean = dataframe_mean(grouped); \
                print(mean); \
                dataframe_free(grouped); \
                dataframe_free(filtered); \
                dataframe_free(df); \
            }",
        );
        assert!(c.contains("typedef struct AgamDataFrame"));
        assert!(c.contains("AgamDataFrame* __v"));
        assert!(!c.contains("/* ── External function stubs ── */"));
    }

    #[test]
    fn test_tensor_runtime_handles_emit() {
        let c = compile_to_c(
            "fn main() { \
                let input = tensor_fill_rand(8, 8, 0.25); \
                let dense = dense_layer(input, 4, 0.5); \
                let conv = conv2d(input, 3, 0.75); \
                let score = tensor_checksum(dense) + tensor_checksum(conv); \
                print(score); \
                tensor_free(conv); \
                tensor_free(dense); \
                tensor_free(input); \
            }",
        );
        assert!(c.contains("typedef struct AgamTensor"));
        assert!(c.contains("AgamTensor* __v"));
        assert!(c.contains("agam_dense_layer"));
        assert!(c.contains("agam_conv2d"));
    }

    #[test]
    fn test_adam_builtin_returns_float() {
        let c = compile_to_c(
            "fn main() { let loss = adam(-1.0, 2.0, 0.001, 64, 0.0001); print(loss); }",
        );
        assert!(c.contains("agam_adam"));
        assert!(c.contains("agam_float __v"));
    }

    #[test]
    fn test_full_pipeline() {
        let c = compile_to_c("fn main(): let x = 10 + 20");
        // Should produce valid-looking C code
        assert!(c.contains("int main("));
        assert!(c.contains("return"));
    }

    #[test]
    fn test_emit_effect_perform_filesystem() {
        let c = compile_to_c("fn main() { perform FileSystem.exists(\".\"); }");
        assert!(
            c.contains("agam_effect_FileSystem_exists"),
            "C output should call agam_effect_FileSystem_exists, not a TODO stub"
        );
        assert!(
            !c.contains("/* TODO: effect"),
            "C output should not contain TODO effect stubs"
        );
    }

    #[test]
    fn test_emit_effect_perform_console() {
        let c = compile_to_c("fn main() { perform Console.println(\"hello\"); }");
        assert!(
            c.contains("agam_effect_Console_println"),
            "C output should call agam_effect_Console_println"
        );
    }

    #[test]
    fn test_effect_prelude_emitted() {
        let c = compile_to_c("fn main(): return 0");
        assert!(
            c.contains("Agam Effect Runtime"),
            "C output should include the effect runtime prelude"
        );
        assert!(
            c.contains("agam_effect_FileSystem_exists"),
            "effect prelude should define FileSystem.exists"
        );
        assert!(
            c.contains("agam_effect_Console_println"),
            "effect prelude should define Console.println"
        );
    }
}
