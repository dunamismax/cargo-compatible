# cargo-compatible

`cargo-compatible` is a Cargo subcommand for checking whether a workspace's resolved dependency graph still fits a target Rust version or MSRV.

It is built around a conservative workflow:

1. scan the current graph
2. try a candidate lockfile in a temporary workspace copy
3. explain the blockers that remain
4. suggest direct manifest changes only when a lockfile-only path is not enough

Crates.io: <https://crates.io/crates/cargo-compatible>

## Install

```bash
cargo install cargo-compatible
```

After installation, use it as `cargo compatible`.

## What it does

- reads the current workspace graph through `cargo metadata --format-version 1`
- selects the target Rust version from `--rust-version`, selected package metadata, or mixed-workspace analysis
- matches `--package` by exact workspace member package name, package ID, or manifest path
- classifies resolved packages as `incompatible` or `unknown`
- shows dependency paths from the selected workspace members to blockers
- builds candidate lockfiles in a temporary workspace without mutating the real checkout
- applies a saved candidate lockfile only on explicit command
- suggests conservative direct dependency requirement changes for crates.io dependencies when lockfile-only resolution is not enough, using either the crates.io sparse cache or a crates.io `local-registry` replacement from `.cargo/config.toml` when available

## Safety model

- `scan` never mutates user files
- `resolve` runs in a temp workspace copy by default
- `apply-lock` requires an explicit candidate lockfile path
- `suggest-manifest` is dry-run by default
- `suggest-manifest --write-manifests` stages validated edits before persisting each file so later failures do not leave earlier manifests half-updated
- missing dependency `rust-version` metadata is treated as `unknown`, not silently compatible

## Command surface

- `cargo compatible scan`
- `cargo compatible resolve`
- `cargo compatible apply-lock`
- `cargo compatible suggest-manifest`
- `cargo compatible explain <crate-or-pkgid>`

## Quick start

```bash
cargo compatible scan --workspace
cargo compatible resolve --workspace --write-candidate .cargo-compatible/candidate/Cargo.lock
cargo compatible explain serde
cargo compatible suggest-manifest --package my-crate
cargo compatible apply-lock --candidate-lockfile .cargo-compatible/candidate/Cargo.lock
```

## Command notes

### `cargo compatible scan`

Use this first to see the current workspace state.

```bash
cargo compatible scan --workspace
cargo compatible scan --package app --format json
cargo compatible scan --rust-version 1.70
```

### `cargo compatible resolve`

`resolve` creates a temp workspace copy, asks stable Cargo for a candidate graph there, and can optionally save both the candidate lockfile and a rendered report. `--write-report` writes the same output selected by `--format`.

```bash
cargo compatible resolve --workspace
cargo compatible resolve --workspace --write-candidate .cargo-compatible/candidate/Cargo.lock
cargo compatible resolve --workspace --write-report compatible-report.md --format markdown
```

### `cargo compatible apply-lock`

Apply a previously saved candidate lockfile back to the real workspace.

```bash
cargo compatible apply-lock --candidate-lockfile .cargo-compatible/candidate/Cargo.lock
```

### `cargo compatible suggest-manifest`

Use this after a best-effort lockfile pass when direct dependency constraints still block the target.

```bash
cargo compatible suggest-manifest --package app
cargo compatible suggest-manifest --package app --write-manifests
cargo compatible suggest-manifest --package app --allow-major
```

### `cargo compatible explain`

Explain why a package is present and why it is incompatible or unknown. Queries must resolve inside the selected dependency graph. When a short name is ambiguous, use a package ID or `name@version`. Human and Markdown reports include lightweight source labels for workspace and path packages, with package IDs as the escape hatch for harder same-name collisions.

```bash
cargo compatible explain serde
cargo compatible explain "serde@1.0.218"
```

## Current limitations

- manifest suggestions are intentionally conservative and currently focus on normal direct dependencies
- manifest suggestions depend on crates.io metadata being locally available, either through the sparse cache or a workspace-level crates.io `local-registry` replacement
- feature validation does not try to fully reimplement Cargo feature resolution semantics
- `resolve` currently favors correctness and safety over speed by re-running Cargo in a full temp copy
- detailed `resolve` version-change reporting stays conservative when multiple resolved versions share the same stable package identity
- resolver guidance for mixed or virtual workspaces is explanatory only; this version does not auto-edit `workspace.resolver`
- path and git dependencies are analyzed and explained, but they do not receive fabricated crates.io downgrade suggestions

## Development

Standard local verification for code changes:

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

For the fuller repo state, progress tracking, and source-of-truth map, read `BUILD.md`.

## License

MIT. See `LICENSE`.
