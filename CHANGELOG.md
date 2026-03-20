# Changelog

## 0.1.0

- Initial release.
- Added `scan`, `resolve`, `apply-lock`, `suggest-manifest`, and `explain`.
- Added workspace-aware Rust version selection and mixed-member analysis.
- Added dry-run lockfile candidate generation in a temp workspace.
- Added conservative manifest suggestion support backed by the local crates.io sparse index cache.
- Added CLI integration tests, snapshot coverage, and CI.
