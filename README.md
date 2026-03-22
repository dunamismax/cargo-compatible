# cargo-compatible

[![CI](https://github.com/dunamismax/cargo-compatible/actions/workflows/ci.yml/badge.svg)](https://github.com/dunamismax/cargo-compatible/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cargo-compatible.svg)](https://crates.io/crates/cargo-compatible)
[![docs.rs](https://docs.rs/cargo-compatible/badge.svg)](https://docs.rs/cargo-compatible)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Cargo subcommand that answers: **"Does my workspace's dependency graph fit the Rust version I care about?"**

`cargo-compatible` audits your resolved dependency graph against a target Rust version or MSRV, shows you exactly what's blocking compatibility, and offers a safe, incremental path to fix it.

## Why this exists

Managing MSRV across a workspace with dozens of dependencies is painful. You bump a dependency, CI breaks on your MSRV target, and now you're spelunking through `cargo tree` output to figure out which transitive dependency dragged in a newer `rust-version` requirement.

`cargo-compatible` solves this by:

1. **Scanning** your current graph and classifying every resolved package as compatible, incompatible, or unknown
2. **Resolving** a candidate lockfile in a sandboxed temp workspace to see if Cargo can find a better solution
3. **Explaining** exactly why specific packages are blockers, with full dependency paths from your workspace members
4. **Suggesting** conservative manifest changes only when a lockfile-only fix isn't enough

The lockfile-first workflow matters: changing `Cargo.lock` is low-risk and reversible. Changing `Cargo.toml` version requirements is a commitment. This tool tries the safe thing first.

## Install

```bash
cargo install cargo-compatible
```

After installation, use it as `cargo compatible`.

## Quick start

```bash
# See where you stand
cargo compatible scan --workspace

# Try to resolve a better lockfile
cargo compatible resolve --workspace --write-candidate .cargo-compatible/candidate/Cargo.lock

# Understand a specific blocker
cargo compatible explain serde

# If lockfile-only isn't enough, get manifest suggestions
cargo compatible suggest-manifest --package my-crate

# Apply the candidate lockfile when you're satisfied
cargo compatible apply-lock --candidate-lockfile .cargo-compatible/candidate/Cargo.lock
```

## Commands

### `cargo compatible scan`

Analyze the current workspace state. This is your starting point — it reads the existing lockfile and classifies resolved packages without changing anything.

```bash
cargo compatible scan --workspace
cargo compatible scan --package app --format json
cargo compatible scan --rust-version 1.70
```

### `cargo compatible resolve`

Build a candidate lockfile in a temporary workspace copy. Your real workspace is never modified. Optionally save the candidate and a rendered report.

```bash
cargo compatible resolve --workspace
cargo compatible resolve --workspace --write-candidate .cargo-compatible/candidate/Cargo.lock
cargo compatible resolve --workspace --write-report report.md --format markdown
```

### `cargo compatible apply-lock`

Apply a previously saved candidate lockfile to the real workspace. Requires an explicit path — no implicit lockfile rewrites.

```bash
cargo compatible apply-lock --candidate-lockfile .cargo-compatible/candidate/Cargo.lock
```

### `cargo compatible suggest-manifest`

Suggest direct dependency requirement changes when lockfile-only resolution isn't enough. Dry-run by default.

```bash
cargo compatible suggest-manifest --package app
cargo compatible suggest-manifest --package app --write-manifests
cargo compatible suggest-manifest --package app --allow-major
```

### `cargo compatible explain`

Explain why a specific package is present and whether it's a compatibility blocker. Shows dependency paths from your workspace members to the queried package.

```bash
cargo compatible explain serde
cargo compatible explain "serde@1.0.218"
```

## Safety model

This tool is designed to be safe to run in any context:

- **`scan`** never mutates user files
- **`resolve`** runs in a temp workspace copy by default — your checkout stays untouched
- **`apply-lock`** requires an explicit candidate lockfile path — no surprise rewrites
- **`suggest-manifest`** is dry-run by default; `--write-manifests` stages and validates all edits before persisting
- **Missing `rust-version`** metadata is treated as unknown, never silently assumed compatible
- **Path and git dependencies** are analyzed and explained but never receive fabricated crates.io downgrade suggestions

## Output formats

All commands support `--format {human|json|markdown}`:

- **human** (default): readable terminal output with source labels and dependency paths
- **json**: machine-readable, suitable for CI integration and downstream tooling
- **markdown**: report-ready format for PRs, issues, or documentation

## How it compares

| Feature | cargo-compatible | cargo-msrv | manual `cargo tree` |
|---|---|---|---|
| Lockfile-first workflow | Yes | No | N/A |
| Sandbox resolution | Yes (temp copy) | No | N/A |
| Dependency path reporting | Yes | No | Manual |
| Manifest suggestions | Conservative, registry-only | Version bisection | Manual |
| Mixed-workspace support | Yes (per-member analysis) | Limited | Manual |
| JSON output | Yes | Yes | No |
| Safety model | Non-mutating by default | Modifies toolchain | Read-only |

## Current limitations

- Manifest suggestions focus on normal direct crates.io dependencies and require locally available sparse-index or local-registry metadata
- Feature validation does not fully reimplement Cargo feature resolution semantics
- `resolve` favors correctness and safety over speed (full temp workspace copy)
- Resolver guidance for mixed or virtual workspaces is explanatory only — no auto-edit of `workspace.resolver`
- Path and git dependencies are analyzed but don't receive downgrade suggestions

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo nextest run
cargo deny check
cargo bench --bench large_workspace_resolver --no-run
```

Tracing is opt-in:

```bash
RUST_LOG=cargo_compatible=debug cargo compatible scan --workspace
```

See [`BUILD.md`](BUILD.md) for the full development manual, phase tracking, and verification ledger. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for development setup and PR guidelines.

## License

MIT. See [`LICENSE`](LICENSE).
