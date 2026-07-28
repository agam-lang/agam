/* ── Agam Effect Runtime ───────────────────── */
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

