# Cargo Compatible Agent Notes

This file is concise repo memory for future agents and developers. Read `README.md` first for the public contract and current operator-facing workflow, then use this file for repo-specific implementation notes.

## Purpose

`cargo-compatible` is a Cargo subcommand for auditing whether a workspace's currently resolved dependency graph fits a target Rust version or MSRV, producing a safer lockfile candidate, explaining blockers, and conservatively suggesting direct manifest changes only when lockfile-only resolution is not enough.

## Vision

Become the standard Cargo companion for MSRV management. The trajectory: v0.1 correctness foundation, v0.2 ecosystem integration (CI output, GitHub Actions, config files), v0.3 intelligence (feature-aware analysis, policy mode), v0.4 performance (incremental, caching), v1.0 production-grade (stable schema, exhaustive coverage). Non-goal: never become a package manager or build system.

## Architecture

```
cli.rs → lib.rs → metadata.rs → compat.rs ─┐
                                             ├→ report.rs (+ identity.rs)
                  resolution.rs ←────────────┘
                  temp_workspace.rs
                  index.rs → manifest_edit.rs
                  explain.rs
```

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
  - crates.io sparse-index or local-registry lookup, compatible candidate selection, and semver/property-test invariants
- `src/manifest_edit.rs`
  - Direct dependency inspection, conservative suggestion generation, and minimal TOML edits
- `src/explain.rs`
  - Per-package explanation assembly and blocker classification
- `src/identity.rs`
  - Package identity labeling, stable comparison keys, and collision fallback helpers
- `src/report.rs`
  - Human, JSON, and Markdown report rendering
- `benches/large_workspace_resolver.rs`
  - Criterion benchmark that generates large path-only workspaces and measures `build_candidate_resolution`
- `tests/integration_cli.rs`
  - Real CLI integration coverage and snapshot tests over fixture workspaces
- `tests/version_selection.rs`
  - Unit coverage for registry candidate selection rules
- `deny.toml`
  - Cargo-deny policy for advisories, licenses, bans, and allowed sources
- `tests/fixtures/*`
  - Deterministic sample workspaces for missing rust-version, mixed-workspace, path dependency, local-registry manifest-blocker, and resolver-guidance cases

## Design Notes

- Missing dependency `rust-version` metadata is treated as unknown, never automatically compatible.
- `scan` and `resolve` are dry-run-first workflows by design.
- `resolve` uses a temporary workspace copy instead of mutating the real checkout.
- Tracing is opt-in through `RUST_LOG`; normal command output remains unchanged unless tracing is enabled.
- Manifest suggestions intentionally prefer conservative no-op behavior when registry metadata is missing or the dependency is non-registry; they can read either the crates.io sparse cache or a crates.io `local-registry` replacement from workspace config.
- `suggest-manifest --write-manifests` stages validated manifest edits before atomically persisting each file so later failures do not leave earlier manifests partially applied.
- Output ordering is deterministic to keep human reports and snapshots stable.
- Human and Markdown reports include lightweight workspace/path/git identity labels in both resolved-package lines and dependency-path chains where they materially reduce same-name package ambiguity; the rare remaining collisions fall back to package IDs.
- `resolve` version-change details stay conservative when the same stable package identity appears at multiple resolved versions; ambiguous identities are noted instead of being collapsed into a misleading single change line.
- `--package` selection is exact by workspace member package name, package ID, or manifest path; avoid relying on substring path matches.
- `explain` only succeeds for packages reachable from the selected dependency graph; out-of-scope queries should fail clearly.
- Path and git dependencies are analyzed and explained, but they do not receive bogus crates.io downgrade suggestions.
- `cargo-deny` currently passes with duplicate-version warnings from transitive dependencies; CI treats them as warnings, not failures.
- MSRV is declared as 1.89 in `Cargo.toml` and verified in CI.

## Verified Commands

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo nextest run
cargo deny check
cargo bench --bench large_workspace_resolver --no-run
cargo run -- --help
cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml
```

## Current Priorities

- Keep CLI help text, README examples, and actual command behavior aligned.
- Preserve the non-mutating-by-default safety contract in both docs and code.
- Treat JSON output as useful but not yet separately versioned; avoid implying a stronger stability contract than the code and docs explicitly guarantee.
- Prefer tightening trust, release hygiene, and operator clarity over broad new feature expansion.

## Current Gaps

- `resolve` relies on stable Cargo commands in a full temp copy of the workspace; Phase 6 benchmarks confirm temp-workspace copy accounts for ~20–25% of resolve time, which is acceptable at current scale (up to 96 members measured).
- Manifest suggestion logic is strongest for normal direct crates.io dependencies and currently relies on locally available sparse-index or local-registry metadata.
- Feature validation is conservative and not a complete reimplementation of Cargo feature resolution semantics.
- Mixed-workspace reasoning is explanatory rather than prescriptive; this version does not auto-edit `workspace.resolver = "3"`.
- The Criterion benchmark harness now covers seven groups (resolve, scan, metadata load, temp-workspace copy, dense graph, mixed version, fixture-derived), but still lacks a registry-backed corpus with real crates.io dependencies.
- JSON output schema is not yet formally versioned or documented.

## Working Agreement

- Keep `README.md`, `CONTRIBUTING.md`, and this file aligned whenever command behavior, output schema, workflow guarantees, or repository structure change.
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
