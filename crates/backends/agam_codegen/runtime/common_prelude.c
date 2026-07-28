/* ── Agam Runtime Prelude ──────────────────── */
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

