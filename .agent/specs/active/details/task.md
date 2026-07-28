# Backup and Push Task Checklist

- [x] Clean up build directories
  - [x] Run `cargo clean` inside `agam` folder
  - [x] Delete `benchmarks/build` directory
  - [x] Delete `agam/dist` directory
  - [x] Delete `agam/.agam_cache` directory
  - [x] Delete `agam/graphify-out` directory
- [x] Commit and Push parent repository changes
  - [x] Check git status of parent repo
  - [x] Commit parent repo changes
  - [x] Push parent repo changes to GitHub
- [x] Commit and Push `agam` sub-repository changes
  - [x] Check git status of `agam` repo
  - [x] Commit `agam` repo changes
  - [x] Push `agam` repo changes to GitHub
- [x] Verify
  - [x] Measure total workspace size (should be under 500 MB)
  - [x] Check git status is clean or only has ignored folders
