# cargo-compatible

[![CI](https://github.com/dunamismax/cargo-compatible/actions/workflows/ci.yml/badge.svg)](https://github.com/dunamismax/cargo-compatible/actions/workflows/ci.yml) [![crates.io](https://img.shields.io/crates/v/cargo-compatible.svg)](https://crates.io/crates/cargo-compatible) [![docs.rs](https://docs.rs/cargo-compatible/badge.svg)](https://docs.rs/cargo-compatible) [![MSRV](https://img.shields.io/badge/MSRV-1.89-blue.svg)](https://github.com/dunamismax/cargo-compatible/blob/main/Cargo.toml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Audit your workspace's dependency graph against any Rust version. Fix what's blocking, safely.**

`cargo-compatible` is a Cargo subcommand that answers "does my resolved dependency graph fit the Rust version I care about?" It scans your lockfile, classifies every package as compatible, incompatible, or unknown, and offers a safe, incremental path to fix blockers — lockfile changes first, manifest edits only when necessary.

> **Status:** v0.1 core is implemented and published on crates.io. The command surface (`scan`, `resolve`, `apply-lock`, `suggest-manifest`, `explain`) ships today, and the active work is performance realism, release polish, and operator trust for v0.2. See [BUILD.md](BUILD.md) for the full execution plan.

## Why cargo-compatible?

Managing MSRV across a workspace with dozens of dependencies is painful. You bump a dependency, CI breaks on your MSRV target, and now you're spelunking through `cargo tree` output to figure out which transitive dependency dragged in a newer `rust-version` requirement.

`cargo-compatible` solves this by:

1. **Scanning** your current graph and classifying every resolved package as compatible, incompatible, or unknown
2. **Resolving** a candidate lockfile in a sandboxed temp workspace to see if Cargo can find a better solution
3. **Explaining** exactly why specific packages are blockers, with full dependency paths from your workspace members
4. **Suggesting** conservative manifest changes only when a lockfile-only fix isn't enough

The lockfile-first workflow matters: changing `Cargo.lock` is low-risk and reversible. Changing `Cargo.toml` version requirements is a commitment. This tool tries the safe thing first.

| Feature | cargo-compatible | cargo-msrv | manual `cargo tree` |
|---|---|---|---|
| Lockfile-first workflow | Yes | No | N/A |
| Sandbox resolution | Yes (temp copy) | No | N/A |
| Dependency path reporting | Yes | No | Manual |
| Manifest suggestions | Conservative, registry-only | Version bisection | Manual |
| Mixed-workspace support | Yes (per-member analysis) | Limited | Manual |
| JSON output | Yes | Yes | No |
| Safety model | Non-mutating by default | Modifies toolchain | Read-only |

## Install

### Prerequisites

- Rust toolchain (stable)

### From crates.io

```bash
cargo install cargo-compatible
```

After installation, use it as `cargo compatible`.

### From source

```bash
git clone https://github.com/dunamismax/cargo-compatible.git
cd cargo-compatible
cargo install --path .
```

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

## Architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                     cargo compatible CLI                      │
│          (clap: scan/resolve/apply-lock/suggest/explain)      │
└──────┬──────────┬──────────┬──────────┬──────────┬───────────┘
       │          │          │          │          │
  ┌────▼───┐ ┌───▼────┐ ┌───▼───┐ ┌───▼────┐ ┌───▼─────┐
  │Metadata│ │ Compat │ │Resolve│ │Manifest│ │ Explain │
  │        │ │        │ │       │ │  Edit  │ │         │
  │ cargo  │ │ graph  │ │ temp  │ │ sparse │ │ blocker │
  │metadata│ │analysis│ │sandbox│ │ index  │ │ paths   │
  └───┬────┘ └───┬────┘ └───┬───┘ └───┬────┘ └───┬─────┘
      │          │          │          │          │
      └──────────┴──────────▼──────────┴──────────┘
                      ┌──────────┐
                      │ Identity │
                      │ + Report │
                      │          │
                      │ human    │
                      │ json     │
                      │ markdown │
                      └──────────┘
```

- **Metadata** — runs `cargo metadata`, identifies workspace/package scope, determines target Rust version
- **Compat** — analyzes the resolved graph, classifies packages, captures dependency paths
- **Resolve** — creates an isolated temp workspace, generates candidate lockfiles, diffs against current state
- **Manifest Edit** — inspects sparse-index or local-registry metadata, produces conservative direct-dependency suggestions
- **Explain** — assembles per-package reasoning with blocker classification and dependency-path context
- **Identity + Report** — renders results in human, JSON, or Markdown form with source-aware labeling

## Repository Layout

```text
.
├── BUILD.md                          # execution manual, phase tracking, verification ledger
├── README.md                         # public-facing project description, honest status
├── AGENTS.md                         # concise repo memory for agents and contributors
├── CONTRIBUTING.md                   # development setup, coding standards, PR process
├── CHANGELOG.md                      # user-facing change history
├── SECURITY.md                       # security policy
├── LICENSE                           # MIT
├── Cargo.toml                        # single-crate package definition
├── Cargo.lock                        # repo lockfile
├── deny.toml                         # dependency-policy checks
├── .editorconfig                     # editor consistency settings
├── .github/
│   └── workflows/ci.yml              # CI gate
├── src/
│   ├── main.rs                       # binary entrypoint + opt-in tracing
│   ├── lib.rs                        # command dispatch and orchestration
│   ├── cli.rs                        # clap command surface and examples
│   ├── model.rs                      # serializable shared analysis types
│   ├── metadata.rs                   # cargo metadata loading, scope selection
│   ├── compat.rs                     # compatibility analysis and dep-path capture
│   ├── resolution.rs                 # candidate lockfile generation and diffing
│   ├── temp_workspace.rs             # temp-copy support for safe resolution
│   ├── index.rs                      # crates.io sparse-index / local-registry lookup
│   ├── manifest_edit.rs              # conservative manifest suggestion and TOML edits
│   ├── explain.rs                    # per-package explanation and blocker classification
│   ├── identity.rs                   # stable package identity labeling
│   └── report.rs                     # human, JSON, and Markdown rendering
├── tests/
│   ├── integration_cli.rs            # snapshot-backed CLI integration coverage
│   ├── version_selection.rs          # focused selection-rule coverage
│   └── fixtures/                     # deterministic sample workspaces
│       ├── missing-rust-version/
│       ├── mixed-workspace/
│       ├── path-too-new/
│       ├── virtual-workspace/
│       └── local-registry-manifest-blocker/
└── benches/
    └── large_workspace_resolver.rs   # Criterion benchmark for synthetic workspace
```

## Current limitations

- Manifest suggestions focus on normal direct crates.io dependencies and require locally available sparse-index or local-registry metadata
- Feature validation does not fully reimplement Cargo feature resolution semantics
- `resolve` favors correctness and safety over speed (full temp workspace copy)
- Resolver guidance for mixed or virtual workspaces is explanatory only — no auto-edit of `workspace.resolver`
- Path and git dependencies are analyzed but don't receive downgrade suggestions

## Roadmap

| Phase | Name | Status |
|-------|------|--------|
| 0 | Repo charter and verification baseline | **Done** |
| 1 | Core command surface and analysis engine | **Done** |
| 2 | Safe resolution and manifest-suggestion workflow | **Done** |
| 3 | Reporting, fixtures, CI, and benchmark baseline | **Done** |
| 4 | Correctness hardening (selection, explain, report) | **Done** |
| 5 | Write-path and mutating-flow coverage | **Done** |
| 6 | Performance realism and benchmark expansion | **Next** |
| 7 | Release polish and operator trust cleanup | Planned |
| 8 | CI/CD hardening and release automation | Planned |
| 9 | Ecosystem integration and interoperability | Planned |
| 10 | Advanced analysis and resolution intelligence | Planned |
| 11 | Documentation, examples, and onboarding | Planned |
| 12 | Community readiness and 1.0 roadmap | Planned |

See [BUILD.md](BUILD.md) for the full phase breakdown with goals, exit criteria, risks, and decisions.

## Design principles

1. **Lockfile first, manifests second.** Changing `Cargo.lock` is low-risk and reversible. Changing `Cargo.toml` is a commitment. The tool always tries the safe thing first.
2. **Non-mutating by default.** Read commands never write. Write commands require explicit flags. No surprises.
3. **Conservative over clever.** If the tool can't prove something, it says "unknown" instead of guessing. No suggestion is better than a bogus one.
4. **Sandbox everything.** Resolution experiments run in isolated temp workspaces. Your checkout is never modified by analysis.
5. **Explain, don't just report.** Blockers come with full dependency paths and reasoning. Users should understand *why*, not just *what*.
6. **Local-first.** All analysis uses locally available metadata. No network calls, no accounts, no cloud dependencies in the core path.
7. **Truthful status.** Docs, CLI output, and code must agree. If they don't, the next change reconciles all three.

## Development And Verification

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

## Contributing

cargo-compatible is actively moving from correctness hardening into performance and release-polish work. Contributions are welcome — see [`CONTRIBUTING.md`](CONTRIBUTING.md) for development setup, coding standards, and PR guidelines. Design feedback and bug reports are always valuable — open an issue.

## License

MIT — see [LICENSE](LICENSE).
