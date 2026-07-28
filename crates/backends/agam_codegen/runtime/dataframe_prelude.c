agam_float agam_adam(agam_float x0, agam_float y0, agam_float learning_rate, agam_int max_iter, agam_float tol) {
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

