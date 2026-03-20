# cargo-compatible

`cargo-compatible` is a Cargo subcommand for finding the highest dependency graph that still fits a chosen Rust version or MSRV.

It focuses on five workflows:

1. `cargo compatible scan`
2. `cargo compatible resolve`
3. `cargo compatible apply-lock`
4. `cargo compatible suggest-manifest`
5. `cargo compatible explain <crate-or-pkgid>`

## What It Does

- Reads the current workspace graph through `cargo metadata --format-version 1`.
- Selects the target Rust version from `--rust-version`, selected package metadata, or mixed-workspace analysis.
- Marks resolved packages as:
  - incompatible when `package.rust-version` is higher than the selected target
  - unknown when `package.rust-version` is missing
- Shows dependency paths from selected workspace members to problematic packages.
- Builds candidate lockfiles in a temporary workspace without mutating your real checkout.
- Applies a saved candidate lockfile only on explicit command.
- Suggests conservative direct dependency requirement changes for crates.io dependencies when lockfile-only resolution is not enough, using Cargo's sparse local registry cache when available.

## Safety Model

- `scan` never mutates user files.
- `resolve` works in a temp copy of the workspace by default.
- `apply-lock` requires an explicit candidate lockfile path.
- `suggest-manifest` is dry-run by default.
- Missing dependency `rust-version` metadata is treated as unknown, not compatible.

## Quick Start

```bash
cargo compatible scan --workspace
cargo compatible resolve --workspace --write-candidate .cargo-compatible/candidate/Cargo.lock
cargo compatible explain serde
cargo compatible suggest-manifest --package my-crate
cargo compatible apply-lock --candidate-lockfile .cargo-compatible/candidate/Cargo.lock
```

## Command Notes

### `cargo compatible scan`

Use this first to see the current workspace state.

```bash
cargo compatible scan --workspace
cargo compatible scan --package app --format json
cargo compatible scan --rust-version 1.70
```

### `cargo compatible resolve`

Creates a temp workspace copy and runs stable Cargo resolution there.

```bash
cargo compatible resolve --workspace
cargo compatible resolve --workspace --write-candidate .cargo-compatible/candidate/Cargo.lock
cargo compatible resolve --workspace --write-report compatible-report.json --format json
```

### `cargo compatible apply-lock`

Writes a previously saved candidate lockfile back to the real workspace.

```bash
cargo compatible apply-lock --candidate-lockfile .cargo-compatible/candidate/Cargo.lock
```

### `cargo compatible suggest-manifest`

Finds remaining direct dependency constraints after a best-effort lockfile-only pass.

```bash
cargo compatible suggest-manifest --package app
cargo compatible suggest-manifest --package app --write-manifests
cargo compatible suggest-manifest --package app --allow-major
```

### `cargo compatible explain`

Explains why a package is present and why it is incompatible or unknown.

```bash
cargo compatible explain serde
cargo compatible explain "serde@1.0.218"
```

## JSON Output Shape

The JSON reports are intentionally structured around stable sections instead of raw command text.

- `scan`
  - `target`
  - `workspace`
  - `package_summaries`
  - `incompatible_packages`
  - `unknown_packages`
  - `notes`
- `resolve`
  - `current`
  - `candidate`
  - `version_changes`
  - `improved_packages`
  - `remaining_blockers`
  - `candidate_lockfile`
  - `notes`
- `suggest-manifest`
  - `candidate_resolution`
  - `manifest_suggestions`
  - `write_manifests`
- `explain`
  - `query`
  - `target`
  - `package`
  - `current_status`
  - `current_reason`
  - `current_paths`
  - `current_rust_version`
  - `candidate_version`
  - `candidate_status`
  - `blocker`
  - `notes`

## Current Limitations

- Manifest suggestions are intentionally conservative and currently focus on normal direct dependencies.
- Manifest suggestions depend on crates.io sparse index entries already present in the local Cargo cache; uncached crates are reported conservatively with no rewrite suggestion.
- Feature validation uses registry feature names and optional dependency feature inference; it does not model every Cargo feature edge case.
- `resolve` currently re-runs Cargo in a full temp copy of the workspace, which favors correctness and safety over speed.
- Resolver guidance for virtual workspaces is surfaced as a recommendation; this version does not auto-edit `workspace.resolver`.
- Path and git dependencies are analyzed and explained, but they do not receive bogus registry downgrade suggestions.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## License

MIT. See `LICENSE`.
