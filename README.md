# cargo-compatible

cargo-compatible is a Cargo subcommand for checking whether a resolved dependency graph still fits a target Rust version or MSRV. It can scan the graph, try a candidate lockfile, explain blockers, and suggest conservative manifest changes when a lockfile-only path is not enough.

## Install

```bash
cargo install cargo-compatible
```

## Commands

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
```

## Notes

- missing `rust-version` metadata is reported as unknown, not silently compatible
- `resolve` works through a temporary workspace copy
- manifest suggestions stay conservative, especially for path and git dependencies
