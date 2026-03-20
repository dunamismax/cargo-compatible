# Cargo Compatible Agent Notes

This file is concise repo memory for future agents and developers. `BUILD.md` is the primary handoff document and should be read first.

## Purpose

`cargo-compatible` is a Cargo subcommand for auditing whether a workspace's currently resolved dependency graph fits a target Rust version or MSRV, producing a safer lockfile candidate, explaining blockers, and conservatively suggesting direct manifest changes only when lockfile-only resolution is not enough.

## Architecture

- `src/cli.rs`
  - Clap CLI definitions for `scan`, `resolve`, `apply-lock`, `suggest-manifest`, and `explain`
- `src/lib.rs`
  - Top-level command dispatch, output routing, and shared command orchestration
- `src/model.rs`
  - Serializable report types, selection context, and shared analysis structs
- `src/metadata.rs`
  - `cargo metadata` loading, workspace/package selection, resolver guidance, and target rust-version selection
- `src/compat.rs`
  - Current graph compatibility analysis, dependency-path collection, and incompatible vs unknown classification
- `src/resolution.rs`
  - Temporary-workspace lockfile resolution, candidate comparison, and atomic lockfile application
- `src/temp_workspace.rs`
  - Safe workspace copy logic for dry-run resolution experiments
- `src/index.rs`
  - crates.io sparse-index cache lookup and compatible candidate selection for manifest suggestions
- `src/manifest_edit.rs`
  - Direct dependency inspection, conservative suggestion generation, and minimal TOML edits
- `src/explain.rs`
  - Per-package explanation assembly and blocker classification
- `src/report.rs`
  - Human, JSON, and Markdown report rendering
- `tests/integration_cli.rs`
  - Real CLI integration coverage and snapshot tests over fixture workspaces
- `tests/version_selection.rs`
  - Unit coverage for registry candidate selection rules
- `tests/fixtures/*`
  - Deterministic sample workspaces for missing rust-version, mixed-workspace, path dependency, and resolver-guidance cases

## Design Notes

- Missing dependency `rust-version` metadata is treated as unknown, never automatically compatible.
- `scan` and `resolve` are dry-run-first workflows by design.
- `resolve` uses a temporary workspace copy instead of mutating the real checkout.
- Manifest suggestions intentionally prefer conservative no-op behavior when registry cache data is missing or the dependency is non-registry.
- Output ordering is deterministic to keep human reports and snapshots stable.
- Path and git dependencies are analyzed and explained, but they do not receive bogus crates.io downgrade suggestions.

## Verified Commands

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run -- --help
cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml
```

## Current Gaps

- `resolve` currently relies on stable Cargo commands in a full temp copy of the workspace, which is safe but can be slower on larger repos.
- Manifest suggestion logic is strongest for normal direct crates.io dependencies and only uses the local sparse index cache.
- Feature validation is conservative and not a complete reimplementation of Cargo feature resolution semantics.
- Mixed-workspace reasoning is explanatory rather than prescriptive; this version does not auto-edit `workspace.resolver = "3"`.

## Working Agreement

- Keep `BUILD.md`, `README.md`, and this file aligned whenever command behavior, output schema, workflow guarantees, or repository structure change.
- Prefer updating fixture workspaces and snapshots when changing analysis or reporting behavior.
- Treat the source of truth in this order:
  - `src/cli.rs`
  - `src/lib.rs`
  - `src/metadata.rs`
  - `src/compat.rs`
  - `src/resolution.rs`
  - `src/manifest_edit.rs`
  - `src/report.rs`
  - `tests/integration_cli.rs`
