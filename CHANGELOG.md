# Changelog

## Unreleased

- Added opt-in `tracing` initialization and instrumentation around metadata loading, workspace analysis, registry candidate selection, and temporary resolution runs.
- Added `cargo-deny` policy configuration in `deny.toml` and CI coverage for dependency-policy checks.
- Switched CI test execution to `cargo-nextest` and added benchmark compilation coverage for the new Criterion harness.
- Added `proptest` invariants for semver candidate selection and resolution diff behavior.
- Added a Criterion benchmark for large synthetic workspaces that exercises `build_candidate_resolution`.
- Updated `BUILD.md`, `README.md`, and `AGENTS.md` to reflect the expanded verification and benchmarking workflow.

## 0.1.0

- Initial release.
- Added `scan`, `resolve`, `apply-lock`, `suggest-manifest`, and `explain`.
- Added workspace-aware Rust version selection and mixed-member analysis.
- Added dry-run lockfile candidate generation in a temp workspace.
- Added conservative manifest suggestion support backed by the local crates.io sparse index cache.
- Added CLI integration tests, snapshot coverage, and CI.
