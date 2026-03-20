# BUILD.md

> **This is the primary operational handoff document for this repository. Keep it current whenever behavior, tooling, docs, workflows, or repository structure change.**

Audited directly from the repository and local command verification on March 20, 2026.

## Current State

`cargo-compatible` is a production-ready Cargo subcommand for analyzing whether a workspace's resolved dependency graph is compatible with a chosen Rust version or MSRV.

The current tool implements:

- `cargo compatible scan`
- `cargo compatible resolve`
- `cargo compatible apply-lock`
- `cargo compatible suggest-manifest`
- `cargo compatible explain <crate-or-pkgid>`
- human, JSON, and Markdown output modes
- workspace and package selection via `--workspace` and `--package`
- target Rust version selection from CLI, package metadata, or mixed-workspace analysis
- incompatible vs unknown dependency classification
- dependency path reporting from selected workspace members
- temporary-workspace candidate lockfile resolution
- atomic candidate lockfile application
- conservative direct manifest suggestion generation for crates.io dependencies using locally cached sparse index metadata

## Key Files

- `Cargo.toml`
  - Crate metadata, dependencies, and license declaration
- `src/main.rs`
  - Thin binary entry point
- `src/lib.rs`
  - Command dispatch and output routing
- `src/cli.rs`
  - Full CLI surface and examples
- `src/model.rs`
  - Shared report and selection model
- `src/metadata.rs`
  - `cargo metadata` integration, package selection, and target rust-version resolution
- `src/compat.rs`
  - Current-state compatibility analysis and dependency path capture
- `src/resolution.rs`
  - Candidate lockfile generation, comparison, and apply flow
- `src/temp_workspace.rs`
  - Safe temp copy support for resolution experiments
- `src/index.rs`
  - Sparse-index cache lookup and compatible version choice logic
- `src/manifest_edit.rs`
  - Direct dependency inspection and minimal TOML rewriting
- `src/explain.rs`
  - Explain report assembly and blocker classification
- `src/report.rs`
  - Human, JSON, and Markdown rendering
- `tests/integration_cli.rs`
  - Snapshot-backed CLI integration tests
- `tests/version_selection.rs`
  - Unit coverage for registry candidate selection rules
- `tests/fixtures/*`
  - Deterministic sample workspaces for key compatibility scenarios
- `.github/workflows/ci.yml`
  - Enforced `fmt`, `clippy`, and `test` workflow

## Fixture Coverage

Fixture workspaces currently cover:

1. missing dependency `rust-version`
2. mixed-rust-version workspace members
3. path dependency with a too-new `rust-version`
4. virtual workspace missing explicit resolver guidance

Snapshot coverage currently exists for:

- human mixed-workspace scan output
- JSON missing-rust-version scan output
- human explain output for a path dependency blocker
- JSON resolve output for virtual workspace guidance

## Verified Build And Run Workflow

All commands below were run directly in `/Users/sawyer/github/cargo-compatible` on March 20, 2026.

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --check` | Success | Formatting clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | Success | No warnings |
| `cargo test` | Success | Unit and integration suites passed |
| `cargo run -- --help` | Success | Confirms direct CLI/help surface |
| `cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml` | Success | Confirms current-state reporting |
| `cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml` | Success | Confirms blocker/path explanation |
| `cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml` | Success | Confirms dry-run suggestion path |

## Config And Runtime Notes

- No environment variables are required for normal `scan`, `resolve`, `apply-lock`, or `explain` operation.
- `cargo metadata` must succeed for the selected workspace.
- `resolve` uses a full temporary copy of the workspace and runs stable Cargo commands there.
- `suggest-manifest` only uses crates.io sparse index entries already present in the local Cargo cache.
- Missing package `rust-version` metadata is surfaced as unknown compatibility, not success.
- `apply-lock` fails safely when the requested candidate lockfile does not exist.

## Source-Of-Truth Notes

Treat these as authoritative before trusting prose docs:

- `src/cli.rs`
  - actual CLI flags and defaults
- `src/lib.rs`
  - command dispatch and command wiring
- `src/metadata.rs`
  - target selection and package/workspace resolution semantics
- `src/compat.rs`
  - incompatible and unknown package classification rules
- `src/resolution.rs`
  - candidate lockfile generation and apply semantics
- `src/manifest_edit.rs`
  - manifest suggestion heuristics and write behavior
- `src/report.rs`
  - output format content and structure
- `tests/integration_cli.rs`
  - stable fixture-backed behavior expectations

## Git Remote Notes

- This repo is intended to mirror the owner's standard dual-push setup:
  - `origin` fetches from GitHub over SSH
  - `origin` pushes to both GitHub and Codeberg over SSH
- Expected remote shape:
  - fetch: `git@github.com-dunamismax:dunamismax/cargo-compatible.git`
  - push: `git@github.com-dunamismax:dunamismax/cargo-compatible.git`
  - push: `git@codeberg.org-dunamismax:dunamismax/cargo-compatible.git`

## License

- `LICENSE` contains the MIT license text.
- `Cargo.toml` declares `license = "MIT"`.

## Current Limitations

1. Manifest suggestions are intentionally conservative.
   - They focus on normal direct crates.io dependencies.
   - If sparse index metadata is not already cached locally, the tool prefers no suggestion over guesswork.
2. Feature checks are approximate.
   - Requested features are compared against index metadata and inferred optional-dependency features.
   - This is useful, but not a full reimplementation of Cargo feature resolution.
3. Resolution experiments favor safety over speed.
   - `resolve` copies the workspace into a temp directory before invoking Cargo.
4. Mixed-workspace guidance is explanatory.
   - The tool reports resolver guidance and unification blockers.
   - It does not auto-edit workspace resolver settings in v1.
5. Path and git dependencies are analysis-only for suggestions.
   - They can be reported as blockers and included in explanations.
   - They do not get fake crates.io downgrade recommendations.

## Next Pass Priorities

1. Add more fixture workspaces for real lockfile-only improvement cases and direct dependency semver blockers.
2. Improve source/registry-aware manifest suggestion coverage for renamed and target-specific dependencies.
3. Tighten explain output for feature-restriction and mixed-workspace blocker distinctions.
4. Expand fixture coverage around path and git dependency reporting.
5. Consider richer machine-readable schema notes in the README if external consumers appear.

## Next-Agent Checklist

1. Read `BUILD.md` first.
2. Then read:
   - `src/cli.rs`
   - `src/lib.rs`
   - `src/metadata.rs`
   - `src/compat.rs`
   - `src/resolution.rs`
   - `src/manifest_edit.rs`
   - `src/report.rs`
   - `tests/integration_cli.rs`
3. Run:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test`
4. If CLI or analysis behavior changed, also run:
   - `cargo run -- --help`
   - `cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml`
   - `cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml`
5. If manifest suggestion behavior changed, also run:
   - `cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml`
6. Keep these files aligned when behavior changes:
   - `BUILD.md`
   - `AGENTS.md`
   - `README.md`
   - fixtures and snapshots as needed
