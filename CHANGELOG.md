# Changelog

## Unreleased

- Added local-registry-backed manifest suggestion lookup plus deterministic end-to-end `suggest-manifest --write-manifests` coverage for a crates.io source-replacement fixture.
- Added clearer human-facing package identity labels for workspace/path packages and resolve version changes so same-name crates are easier to distinguish in reports.
- Tightened `--package` to use exact workspace member name/package-ID/manifest-path matching instead of manifest-path substring matches.
- Made `explain` reject queries outside the selected dependency graph and clarified ambiguous-query guidance.
- Made `resolve --write-report` write the same rendered format selected by `--format`, and added direct integration coverage for report/candidate write flows.
- Added direct `apply-lock` coverage, direct manifest-write coverage, and failure-path coverage for missing candidate lockfiles and partial manifest-apply scenarios.
- Hardened package identity handling by avoiding collapsed multi-version resolve diffs for the same package identity and by matching manifest suggestions on package name plus source instead of name alone.
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
