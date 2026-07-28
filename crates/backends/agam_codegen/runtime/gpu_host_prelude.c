/* ── Agam GPU Host Runtime ─────────────────── */
void* agam_gpu_malloc(agam_int size_bytes) {
  if (size_bytes <= 0) {
    return NULL;
  }
  return malloc((size_t)size_bytes);
}

agam_int agam_gpu_free(void* ptr) {
  free(ptr);
  return 0;
}

agam_int agam_gpu_memcpy_to_device(void* device_dst, void* host_src, agam_int size_bytes) {
  if (!device_dst || !host_src || size_bytes < 0) {
    return -1;
  }
  memcpy(device_dst, host_src, (size_t)size_bytes);
  return 0;
}

agam_int agam_gpu_memcpy_to_host(void* host_dst, void* device_src, agam_int size_bytes) {
  if (!host_dst || !device_src || size_bytes < 0) {
    return -1;
  }
  memcpy(host_dst, device_src, (size_t)size_bytes);
  return 0;
}

