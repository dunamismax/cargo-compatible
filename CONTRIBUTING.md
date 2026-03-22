# Contributing to cargo-compatible

Thanks for your interest in contributing. This document covers the development setup, coding standards, and PR process.

## Development setup

### Prerequisites

- Rust stable toolchain (install via [rustup](https://rustup.rs/))
- `cargo-nextest` for test execution: `cargo install cargo-nextest`
- `cargo-deny` for dependency policy checks: `cargo install cargo-deny`
- `cargo-insta` for snapshot review (optional): `cargo install cargo-insta`

### Building

```bash
git clone https://github.com/dunamismax/cargo-compatible.git
cd cargo-compatible
cargo build
```

### Running the full quality gate

Every code change should pass the local gate before submission:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo nextest run
cargo deny check
cargo bench --bench large_workspace_resolver --no-run
```

If you're changing command behavior, also run the smoke gate:

```bash
cargo run -- --help
cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml
```

## Project structure

```
src/
├── main.rs            # Entry point, tracing setup
├── lib.rs             # Command dispatch and orchestration
├── cli.rs             # Clap CLI definitions
├── model.rs           # Shared types and report structures
├── metadata.rs        # Workspace loading and selection
├── compat.rs          # Compatibility analysis
├── resolution.rs      # Lockfile resolution and diffing
├── temp_workspace.rs  # Safe temp-copy logic
├── index.rs           # Registry lookup
├── manifest_edit.rs   # Manifest suggestion and editing
├── explain.rs         # Per-package explanation
├── identity.rs        # Package identity labeling
└── report.rs          # Output rendering (human/JSON/markdown)

tests/
├── integration_cli.rs  # CLI integration tests (snapshot-backed)
├── version_selection.rs # Version selection unit tests
├── snapshots/          # Insta snapshot files
└── fixtures/           # Deterministic test workspaces

benches/
└── large_workspace_resolver.rs  # Criterion benchmark
```

## Coding standards

### Architecture boundaries

- `main.rs` stays thin — entry point only.
- `lib.rs` orchestrates — no domain logic.
- Selection logic stays in `metadata.rs` — no ad-hoc reimplementation per command.
- Compatibility analysis stays distinct from resolution experiments.
- Reporting transforms already-computed structures — no re-running business logic.

### Safety invariants

These are non-negotiable design constraints:

1. `scan` never mutates user files.
2. `resolve` uses a temp workspace copy — never modifies the real checkout.
3. `apply-lock` requires an explicit candidate path — no implicit rewrites.
4. Missing `rust-version` is `unknown`, never silently compatible.
5. Path/git dependencies don't get fabricated crates.io suggestions.
6. `suggest-manifest --write-manifests` stages all edits before persisting.

### Code style

- Run `cargo fmt` before committing.
- All clippy lints must pass with `-D warnings`.
- Prefer returning `anyhow::Result` for fallible operations in the command dispatch layer.
- Use `thiserror` for domain-specific errors that callers need to match on.
- Keep functions short and focused. If a function needs a comment explaining what it does, it should probably be its own function with a descriptive name.
- Output ordering must be deterministic for snapshot stability.

### Testing

- Prefer fixture-backed integration tests over unit tests for I/O and CLI behavior.
- Use `insta` snapshots for output verification — run `cargo insta review` after updating expected output.
- Use `proptest` for invariant testing on core algorithms (candidate selection, diff computation).
- Use `assert_cmd` and `assert_fs` for CLI integration tests with temp directories.
- Every write-path change needs direct test coverage or explicitly recorded manual verification.

### Adding a new fixture

Test fixtures live in `tests/fixtures/`. Each fixture is a minimal Cargo workspace:

1. Create a directory under `tests/fixtures/` with a descriptive name.
2. Add the minimal `Cargo.toml` and source files needed to reproduce the scenario.
3. Ensure the fixture is deterministic — no network access, no timestamp dependencies.
4. Add integration tests in `tests/integration_cli.rs` that exercise the fixture.
5. Run `cargo insta review` to accept new snapshots.

## Pull request process

1. **Fork and branch**: Create a feature branch from `main`.
2. **Pass the gate**: Run the full quality gate locally before pushing.
3. **Small, focused changes**: One logical change per PR. If you're fixing a bug and adding a feature, that's two PRs.
4. **Update docs**: If you change command behavior, output format, or semantics, update `README.md`, `AGENTS.md`, and `BUILD.md` in the same PR.
5. **Update snapshots**: If output changes, run `cargo insta review` and include the updated snapshots.
6. **Write a clear description**: Explain what changed, why, and how to verify it.

### Commit messages

- Use imperative mood: "Add feature" not "Added feature" or "Adds feature".
- Keep the first line under 72 characters.
- Reference issues where applicable.

### What makes a good PR

- Passes CI.
- Has test coverage for new behavior.
- Doesn't break existing snapshots without explanation.
- Keeps docs aligned with code.
- Follows the architecture boundaries described above.

## Reporting issues

When filing a bug report, include:

- Rust version (`rustc --version`)
- `cargo-compatible` version (`cargo compatible --version` or the git commit)
- The command you ran
- The output you got
- The output you expected
- A minimal reproduction workspace if possible

## Questions?

Open a [discussion](https://github.com/dunamismax/cargo-compatible/discussions) or file an [issue](https://github.com/dunamismax/cargo-compatible/issues).
