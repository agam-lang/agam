AgamTensor* agam_tensor_fill_rand(agam_int rows, agam_int cols, agam_float seed) {
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

