# BUILD.md

## Purpose

This file is the canonical execution and tracking document for `cargo-compatible`.

It keeps the repo honest while the project turns a proven v0.1 core into a faster, clearer, more adoptable tool. At any point it should answer:

- what cargo-compatible is trying to become
- what exists right now
- what is explicitly not built yet
- what the next correct move is
- what must be proven before stronger claims are made

Any agent making substantial changes to code, docs, workflow, tests, or command behavior should read it first and update it before handoff. `README.md` stays public-facing and concise; this file carries the deeper repo-operational picture, source-of-truth map, progress state, verification ledger, and next-work guidance.

This is a living document. When code and docs disagree, fix them together in the same change.

---

## Mission

- Make it practical to answer: "does this workspace's resolved graph fit the Rust version I care about?"
- Provide a safe lockfile-first workflow before suggesting manifest edits.
- Explain blockers clearly enough that users can act without reverse-engineering Cargo metadata by hand.
- Stay conservative when the tool cannot prove something, especially around missing `rust-version`, sparse-index gaps, non-registry dependencies, and manifest rewriting.
- Keep the codebase small, auditable, and trustworthy as a Cargo subcommand rather than drifting into a speculative package manager clone.

---

## Long-term vision

`cargo-compatible` aims to become the standard Cargo companion for MSRV management across the Rust ecosystem. The trajectory:

1. **v0.1 — Correctness foundation** (current): prove the analysis engine is trustworthy on real workspaces.
2. **v0.2 — Ecosystem integration**: CI-native output, GitHub Actions, `cargo-deny` interop, lockfile-only downgrade.
3. **v0.3 — Intelligence**: smarter resolution strategies, feature-aware analysis, workspace-level policy.
4. **v0.4 — Performance**: incremental analysis, caching, parallel resolution.
5. **v1.0 — Production-grade**: stable CLI contract, SemVer output schema, exhaustive edge-case coverage.

The non-goal boundary is equally important: this tool should never become an alternative package manager, a build system, or a CI orchestrator. It reads Cargo metadata and lockfiles, experiments in sandboxes, and reports findings. It does not own the build.

---

## Repo snapshot

**Current execution window: Phase 6 complete, Phase 7 active — measured baselines captured, now polishing the operator-facing surface for a credible v0.2 release path.**

What exists today:
- Full command surface: `scan`, `resolve`, `apply-lock`, `suggest-manifest`, `explain`
- Fixture-backed integration tests and snapshot coverage
- CI gate with fmt, clippy, cross-platform nextest/doc tests, MSRV, cargo-deny, dogfood scan, and benchmark compilation
- Human, JSON, and Markdown output modes
- Published on crates.io as `cargo-compatible` 0.1.0

What Phases 4–5 proved:
- Package-identity disambiguation is materially better in dependency-path chains and explain reports
- High-risk write paths now have direct coverage instead of hand-wavy confidence
- The repo has a true lockfile-only improvement fixture backed by a deterministic local registry
- `resolve` now actually upgrades compatible dependencies in the temp workspace by using `cargo update` without `--workspace`

What Phase 6 established:
- Seven benchmark groups covering metadata load, scan analysis, temp-workspace copy, end-to-end resolve, dense graphs, mixed-version workspaces, and fixture-derived scans
- Phase-separated timing proves metadata loading (external `cargo metadata`) dominates resolve cost; temp-workspace copy is ~20–25% of total
- The temp-workspace safety model is justified by measurement; no optimization needed at current scale
- Memory pressure is not a concern at 96-member synthetic workspaces
- CI performance thresholds deferred due to benchmark noise; revisit when corpus grows

What is actively being driven next:
- Operator-trust polish: help text, docs, sample outputs, and error semantics that match the shipped behavior exactly
- Release automation choices for a credible v0.2 story rather than a forever-0.1 holding pattern

What does **not** exist yet:
- Formal JSON output schema versioning
- Binary release automation or crates.io publish-on-tag workflow
- Feature-aware compatibility analysis
- Project-level config file support

### Active code and test surfaces

- `src/main.rs`
  - thin binary entrypoint plus opt-in tracing subscriber initialization
- `src/lib.rs`
  - command dispatch, orchestration, and stdout/file output routing
- `src/cli.rs`
  - Clap command surface and examples
- `src/model.rs`
  - serializable shared analysis and report types
- `src/metadata.rs`
  - `cargo metadata` loading, workspace/package selection, and target Rust version selection
- `src/compat.rs`
  - current-graph compatibility analysis and dependency-path capture
- `src/resolution.rs`
  - candidate lockfile generation, comparison, and apply flow
- `src/temp_workspace.rs`
  - temp-copy support for safe resolution experiments
- `src/index.rs`
  - crates.io sparse-index or local-registry lookup and compatible-version selection
- `src/manifest_edit.rs`
  - conservative direct dependency suggestion and TOML edits
- `src/explain.rs`
  - per-package explanation assembly and blocker classification
- `src/identity.rs`
  - stable package identity labeling and collision fallback helpers for human-facing output
- `src/report.rs`
  - human, JSON, and Markdown rendering
- `tests/integration_cli.rs`
  - snapshot-backed CLI integration coverage
- `tests/version_selection.rs`
  - focused selection-rule coverage
- `tests/fixtures/*`
  - deterministic sample workspaces for missing rust-version, mixed-workspace, path dependency, local-registry manifest-blocker, lockfile-improvement, and resolver-guidance cases
- `benches/large_workspace_resolver.rs`
  - Criterion benchmark for large synthetic workspace resolution

### Implemented command surface

- `cargo compatible scan`
- `cargo compatible resolve`
- `cargo compatible apply-lock`
- `cargo compatible suggest-manifest`
- `cargo compatible explain <crate-or-pkgid>`

### Current behavior snapshot

The shipped workflow currently supports:

- workspace and package scoping via `--workspace` and `--package`, with exact workspace-member matching by package name, package ID, or manifest path
- target Rust version selection from CLI, selected package metadata, or mixed-workspace analysis
- incompatible vs unknown dependency classification
- dependency-path reporting from selected workspace members
- temp-workspace candidate lockfile resolution
- atomic candidate lockfile application
- conservative manifest suggestions for crates.io direct dependencies using locally cached sparse-index metadata or a workspace-local crates.io `local-registry` replacement
- staged `--write-manifests` application with per-file atomic persistence after validation
- human, JSON, and Markdown output modes, including `resolve --write-report` following the selected `--format`
- lightweight source labels in human and Markdown reports for workspace/path packages, with package-ID fallback when same-name collisions remain ambiguous
- conservative resolve diff reporting that omits ambiguous multi-version same-identity resolve diffs instead of collapsing them incorrectly
- opt-in tracing via `RUST_LOG`
- `explain` queries limited to packages reachable from the selected dependency graph
- property tests for candidate-selection and resolution-diff invariants
- CI and local checks using `fmt`, `clippy`, `test`, `nextest`, `cargo-deny`, and benchmark compilation

### Remaining work shape

This repository is past greenfield invention and now has measured performance baselines. The next stretch is about making the tool easier to adopt and harder to misread:

- tighten the operator experience so CLI help, docs, reports, and errors all tell the same story
- turn the already-good CI foundation into a repeatable release path with artifacts and stronger adoption hooks
- decide which advanced analysis features actually earn their complexity before v0.2 scope balloons

---

## Source-of-truth mapping

| File | Owns |
|------|------|
| `README.md` | Public-facing project description, honest status |
| `BUILD.md` | Implementation map, phase tracking, decisions, verification ledger |
| `AGENTS.md` | Concise repo memory for future contributors and agents |
| `CONTRIBUTING.md` | Development setup, coding standards, PR process |
| `CHANGELOG.md` | User-facing change history (Keep a Changelog format) |
| `SECURITY.md` | Security policy and responsible disclosure |
| `Cargo.toml` | Single-crate package definition, dependency manifest |
| `deny.toml` | Dependency-policy checks |
| `.github/workflows/ci.yml` | CI gate definition |
| `src/cli.rs` | Clap command surface, flags, examples |
| `src/lib.rs` | Command dispatch, orchestration, output routing |
| `src/metadata.rs` | Package selection, workspace-member matching, target Rust version |
| `src/compat.rs` | Compatibility analysis, dependency-path reporting |
| `src/resolution.rs` | Candidate lockfile generation, diffing, apply semantics |
| `src/temp_workspace.rs` | Temp-copy support for safe resolution |
| `src/index.rs` | Sparse-index / local-registry lookup, version selection |
| `src/manifest_edit.rs` | Conservative manifest suggestion and TOML edits |
| `src/explain.rs` | Per-package explanation and blocker classification |
| `src/identity.rs` | Package identity labeling and ambiguity handling |
| `src/report.rs` | Human, JSON, and Markdown rendering |
| `tests/integration_cli.rs` | Snapshot-backed CLI integration coverage |
| `tests/version_selection.rs` | Focused selection-rule coverage |
| `benches/large_workspace_resolver.rs` | Performance benchmark scope |

**Invariant:** If docs, code, and CLI output ever disagree, the next change must reconcile all three.

---

## Architecture and flow

### Command and data flow

1. `src/cli.rs` parses the command surface and flags.
2. `src/lib.rs` dispatches to the selected workflow.
3. `src/metadata.rs` runs `cargo metadata`, identifies the selected workspace/package scope, and determines the target Rust version.
4. `src/compat.rs` analyzes the currently resolved graph and records incompatible and unknown packages plus dependency paths.
5. Depending on the command:
   - `src/resolution.rs` and `src/temp_workspace.rs` build a candidate lockfile in a temp workspace and compare it against the current state.
   - `src/index.rs` and `src/manifest_edit.rs` inspect cached sparse-index or local-registry metadata and produce conservative direct-dependency suggestions.
   - `src/explain.rs` assembles per-package reasoning and candidate or blocker context.
6. `src/identity.rs` and `src/report.rs` render the result in human, JSON, or Markdown form without recomputing core analysis.
7. `src/main.rs` optionally enables tracing when `RUST_LOG` is set.

### Boundary intent

- `main.rs` should stay thin.
- `lib.rs` should orchestrate, not absorb domain logic.
- selection logic should stay centralized in `metadata.rs` rather than being reimplemented ad hoc per command.
- compatibility analysis should stay distinct from resolution experiments.
- manifest suggestion logic should remain conservative and separable from lockfile resolution.
- identity labeling and reporting should clarify ambiguity, not invent false precision.
- reporting should transform already-derived structures, not re-run business logic.

---

## Working rules

1. Read `BUILD.md`, then `README.md`, then `AGENTS.md` before substantial work.
2. Trust the source files and fixture-backed tests over prose if there is any mismatch.
3. Preserve the safety model:
   - `scan` stays non-mutating.
   - `resolve` stays temp-copy-first unless a measured decision explicitly changes that.
   - `apply-lock` and manifest writes stay explicit and narrow.
4. Missing dependency `rust-version` metadata stays `unknown`, not silently compatible.
5. Path and git dependencies may be analyzed and explained, but they should not receive fabricated crates.io downgrade suggestions.
6. If command behavior, output schema, or semantics change, keep `BUILD.md`, `README.md`, `AGENTS.md`, `CHANGELOG.md`, fixtures, and snapshots aligned.
7. Do not mark work done unless the artifact exists and the verification section records commands that actually ran.
8. Prefer fixture-backed tests and temp-dir integration tests over hand-wavy reasoning for I/O, selection, or write-path behavior.
9. When a new ambiguity appears, record it in the open-decision or risk sections instead of letting it stay implicit.

---

## Tracking conventions

Use this language consistently in docs, commits, and issues:

| Term | Meaning |
|------|---------|
| **done** | Implemented and verified |
| **checked** | Verified by command or test output |
| **planned** | Intentional, not started |
| **ready** | Scoped well enough that it should be the next execution lane |
| **queued** | Important follow-on work with known prerequisites or sequencing |
| **in-progress** | Actively being worked on |
| **blocked** | Cannot proceed without a decision or dependency |
| **risk** | Plausible failure mode that could distort the design |
| **decision** | A durable call with consequences |

When new work lands, update: repo snapshot, phase dashboard, decisions (if architecture changed), and progress log with date and what was verified.

---

## Quality gates

### Enforced CI gate

From `.github/workflows/ci.yml`:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo deny check
cargo nextest run
cargo bench --bench large_workspace_resolver --no-run
```

### Practical local gate

Use this when making meaningful code changes unless the task is docs-only:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo nextest run
cargo-deny check
cargo bench --bench large_workspace_resolver --no-run
```

### Command-change smoke gate

When CLI, analysis, or reporting behavior changes, also run the smallest relevant command-surface checks from the verified set:

```bash
cargo run -- --help
cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml
```

### Write-path expectation

If work changes `apply-lock`, `--write-candidate`, `--write-report`, or `--write-manifests`, the change is not really done until there is direct temp-dir integration coverage or explicitly recorded manual verification.

For docs-only changes, verify wording consistency and that repo state matches documented commands. If a gate is temporarily unavailable, document why. Never silently skip.

---

## Dependency strategy

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `cargo_metadata` | `cargo metadata` loading and workspace introspection |
| `crates-index` | Sparse-index / local-registry lookup for version selection |
| `semver` | Rust version and SemVer comparison |
| `serde` + `serde_json` | Serialization for JSON output and internal data |
| `toml_edit` | Manifest TOML editing with format preservation |
| `petgraph` | Dependency graph construction and traversal |
| `itertools` | Iterator utilities for collection processing |
| `regex` | Pattern matching in command parsing |
| `anyhow` + `thiserror` | Error handling and propagation |
| `tempfile` | Sandbox directory creation for safe resolution |
| `walkdir` | Directory traversal |
| `tracing` + `tracing-subscriber` | Opt-in structured logging via `RUST_LOG` |

### Dev dependencies

| Crate | Purpose |
|-------|---------|
| `assert_cmd` + `assert_fs` + `predicates` | CLI integration test infrastructure |
| `insta` | Snapshot testing for CLI output |
| `criterion` | Performance benchmarking |
| `proptest` | Property-based testing |
| `sha2` | Hashing for test determinism |

Every dependency addition must be justified in the decisions log with: what it replaces, what it costs, and whether a lighter alternative exists.

---

## Phase dashboard

### Recently completed

- Phase 0 - repo charter, source-of-truth mapping, and verification baseline. Status: **done**.
- Phase 1 - core command surface and analysis engine. Status: **done**.
- Phase 2 - safe resolution and manifest-suggestion workflow. Status: **done**.
- Phase 3 - reporting, fixtures, CI, and benchmark baseline. Status: **done**.
- Phase 4 - correctness hardening for package selection, explain scope, and report semantics. Status: **done**.
- Phase 5 - write-path and mutating-flow coverage. Status: **done**.
- Phase 6 - performance realism and benchmark expansion. Status: **done**.

### Current execution window

- Phase 7 - release polish and operator trust cleanup. Status: **ready**.
- Phase 8 - CI/CD hardening and release automation. Status: **queued** with meaningful groundwork already **done**.

### Forward roadmap

- Phase 9 - ecosystem integration and interoperability. Status: **planned**.
- Phase 10 - advanced analysis and resolution intelligence. Status: **planned**.
- Phase 11 - documentation, examples, and onboarding. Status: **planned**.
- Phase 12 - community readiness and 1.0 roadmap. Status: **planned**.

---

## Phase detail

### Phase 6 - performance realism and benchmark expansion

Status: **done**

Goal: replace hand-wavy performance assumptions with measurements that reflect how people will actually run `cargo-compatible`.

Why this is next:
- The correctness surface is strong enough that performance is now the most likely trust drag on larger repos.
- The temp-workspace safety model is worth keeping unless measurements prove it is the bottleneck.
- Release polish is easier to do honestly once the runtime characteristics are known.

Work completed:

- [x] Add a second benchmark harness that reuses real fixture-derived workspaces instead of only generated path-only trees.
- [x] Split timing capture so metadata loading, compatibility analysis, temp-workspace copy, and resolver invocation can be measured separately.
- [x] Benchmark `scan` and `resolve` against at least three scales: a small fixture workspace, the current large synthetic workspace, and a registry-backed or feature-heavier scenario.
- [x] Measure whether the full temp-workspace copy is materially more expensive than the resolver work itself before considering optimization.
- [x] Record baseline numbers directly in this file so future changes can be compared against something real.
- [x] Profile memory growth on larger synthetic workspaces and note any obvious pressure points.
- [x] Decide whether CI should enforce any performance threshold now or wait until benchmark noise is better understood.

Exit criteria (all met):

- [x] At least two benchmark scenarios exist beyond the current path-only synthetic case.
- [x] `scan` and `resolve` each have attributable timing baselines, not just a single aggregate number.
- [x] Any proposed optimization is justified by measured evidence, not discomfort with the current design.

### Phase 6 benchmark baselines

Captured on macOS arm64 (Apple Silicon), 2026-03-24. All times are Criterion median values from the `large_workspace_resolver` bench harness.

#### Phase-separated timing (synthetic path-only linear workspace)

| Phase | 32 members | 96 members |
|-------|-----------|-----------|
| `metadata_load` | 51.7 ms | 75.9 ms |
| `scan_analysis` | 1.53 ms | 39.4 ms |
| `temp_workspace_copy` | 17.7 ms | 40.6 ms |
| `resolve_end_to_end` | 68.0 ms | 187.6 ms |

#### Workspace topology variants (scan-only)

| Scenario | 32 members | 64/96 members |
|----------|-----------|--------------|
| Linear chain (scan) | 1.53 ms | 39.4 ms (96) |
| Dense graph (up to 8 deps each) | 661 µs | 3.37 ms (64) |
| Mixed rust-version (⅓ 1.70, ⅓ 1.80, ⅓ missing) | 1.64 ms | 41.1 ms (96) |

#### Fixture-derived scan (real test workspaces, pre-loaded metadata)

| Fixture | Time |
|---------|------|
| `path-too-new` (1 member, 1 dep) | 2.69 µs |
| `mixed-workspace` (3 members) | 4.24 µs |
| `missing-rust-version` (2 members) | 2.40 µs |
| `virtual-workspace` (2 members) | 2.08 µs |

#### Key findings

- **Metadata loading dominates resolve cost**: `cargo metadata` accounts for ~50–75 ms of the total, making it the single largest phase. This is external to this tool and not directly optimizable.
- **Temp-workspace copy is material but not a release blocker**: at ~18–41 ms (20–25% of resolve), the safety model is justified. Optimization would require unsafe in-place mutation of the real workspace or incremental diffing, neither of which is worth the complexity for path-only workspaces.
- **Scan analysis itself is fast**: sub-2 ms for 32-member workspaces, scaling to ~40 ms at 96 members in linear-chain topology. Dense graphs are actually faster because BFS traversal visits fewer unique paths.
- **Mixed-version and unknown-member workspaces** do not significantly change analysis cost compared to uniform workspaces of the same size.
- **No memory pressure observed**: all benchmarks complete cleanly at 96 members with no notable allocation spikes or OOM risk.
- **CI performance thresholds deferred**: benchmark noise at the 10-sample level is too high (~5–10% variance) for meaningful regression detection. A performance gate would produce false positives. Revisit when a heavier registry-backed corpus is added or when absolute runtime exceeds operator tolerance.

#### Benchmark harness inventory

The `large_workspace_resolver` harness now contains seven benchmark groups:

1. `resolve_end_to_end` — full resolution pipeline (original, expanded)
2. `scan_analysis` — compatibility analysis only, metadata pre-loaded
3. `metadata_load` — isolated `cargo metadata` invocation cost
4. `temp_workspace_copy` — isolated filesystem copy cost
5. `dense_graph_scan` — scan with denser inter-member dependency topology
6. `mixed_version_scan` — scan with heterogeneous rust-version declarations
7. `fixture_scan` — scan using real test fixtures (path-too-new, mixed-workspace, missing-rust-version, virtual-workspace)

### Phase 7 - release polish and operator trust cleanup

Status: queued

Goal: make the first-run experience sharp enough that the repo reads like a tool people can adopt immediately, not just a promising prototype.

Why this follows Phase 6:
- Performance claims and examples should not get ahead of the measurements.
- The docs/help/error pass should describe the real product, not the hoped-for one.
- This phase is where the public surface gets tightened before broader CI and ecosystem integrations land.

Work to do:

- [ ] Re-run every README command example against the current CLI and fix or delete anything aspirational.
- [ ] Audit `--help` text for every subcommand so flags, defaults, and safety notes match actual behavior.
- [ ] Add at least one concise end-to-end sample output for `scan`, `resolve`, and `explain` that reflects real snapshots.
- [ ] Tighten error messages around missing registry metadata, unreachable `explain` targets, and write-path preconditions so users get a next action, not just a failure.
- [ ] Document when and why package IDs appear as the fallback identity in human-facing output.
- [ ] Decide whether the JSON schema is versioned in v0.2 or explicitly marked unstable until v0.3.
- [ ] Decide whether shell completions belong in the v0.2 scope or should wait until the output contract is more settled.

Exit criteria:

- [ ] A new user can install the crate, run the README quick start, and get exactly the behavior the docs promise.
- [ ] The help surface is internally consistent across README, clap help text, and emitted reports.
- [ ] The JSON output stability story is explicit rather than implied.

### Phase 8 - CI/CD hardening and release automation

Status: planned (foundation partly done)

Goal: turn the repo's current verification discipline into a repeatable, low-drama release path.

Work to do:

- [x] Add MSRV verification to CI (test against the declared `rust-version` in `Cargo.toml`).
- [x] Add cross-platform CI matrix (Linux, macOS, Windows).
- [x] Add CI job that runs the tool against its own workspace as a dogfood gate.
- [ ] Add a release workflow that publishes to crates.io on tag push with changelog validation.
- [ ] Add binary release builds for common platforms (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64) via `cargo-dist` or explicit cross-compilation.
- [ ] Add `cargo-audit` to CI for security advisory checks beyond `cargo-deny`.
- [ ] Add integration coverage that runs the tool against a known external workspace, not only local fixtures.
- [ ] Decide whether `cargo-semver-checks` is worth the complexity for a primarily CLI-focused crate.
- [ ] Add dependabot or renovate for regular dependency update pressure.

Exit criteria:

- [ ] Tagging a release can produce a publishable crate and binary artifacts without ad hoc shell work.
- [ ] CI catches platform, MSRV, and dogfood regressions before release day.
- [ ] Security and maintenance automation are good enough that a v0.2 release does not depend on heroic memory.

### Phase 9 - ecosystem integration and interoperability

Status: planned

Goal: make `cargo-compatible` fit naturally into CI systems, PR review flows, and adjacent Rust tooling.

Work to do:

- [ ] Add `--exit-code` mode for CI usage (exit 1 if incompatible packages are found, exit 0 if clean).
- [ ] Add GitHub Actions integration (official action or a documented workflow snippet).
- [ ] Add SARIF or SARIF-like output for GitHub Code Scanning integration.
- [ ] Add `cargo-deny` interop: consume relevant policy ideas or emit compatible output where that helps.
- [ ] Add `--diff` mode that compares two lockfiles and reports compatibility changes for PR review.
- [ ] Quantify crates.io `rust-version` coverage to explain how common `unknown` results really are.
- [ ] Add project-level config support via `.cargo-compatible.toml` for common defaults.
- [ ] Add JUnit XML output for CI systems that consume test-style artifacts.
- [ ] Consider editor or LSP integration only if the CLI/report contracts stay clean enough to support it.

Exit criteria:

- [ ] The tool can be dropped into a CI pipeline with a small, well-documented invocation.
- [ ] At least one machine-oriented output contract beyond JSON is production-ready.
- [ ] Project-level configuration measurably reduces flag noise for repeat users.

### Phase 10 - advanced analysis and resolution intelligence

Status: planned

Goal: make the analysis engine smarter without violating the core safety contract.

Work to do:

- [ ] Implement feature-aware compatibility analysis where disabled features materially change compatibility.
- [ ] Add `--ignore` support for intentionally excluded crates or known false positives.
- [ ] Add workspace-level policy modes such as "all members must support 1.70+".
- [ ] Investigate whether `cargo tree` can complement `cargo metadata` for richer dependency reasoning.
- [ ] Add transitive pinning suggestions when a lockfile-only fix exists but needs explicit constraints.
- [ ] Add workspace-aware `rust-version` alignment suggestions across members.
- [ ] Investigate incremental analysis when only `Cargo.lock` changes.
- [ ] Add an `--upgrade-path` mode that shows the minimum set of changes to reach a target Rust version.
- [ ] Consider a `--simulate` mode that predicts the effect of a Rust version bump without rewriting files.

Exit criteria:

- [ ] Feature-aware analysis reduces false positives measurably on real workspaces.
- [ ] At least one suggestion mode goes beyond current direct-dependency conservatism without becoming speculative.
- [ ] All advanced modes preserve the non-mutating-by-default safety model.

### Phase 11 - documentation, examples, and onboarding

Status: planned

Goal: make the tool understandable to Rust developers who are not Cargo internals specialists.

Work to do:

- [ ] Write a user guide (mdbook or standalone markdown) covering common workflows end to end.
- [ ] Add annotated example outputs for each command.
- [ ] Add a troubleshooting section covering common failure modes and likely fixes.
- [ ] Add a concepts page explaining MSRV, `rust-version`, resolver behavior, and why lockfile-first matters.
- [ ] Create cloneable learning fixtures or example repos that users can run locally.
- [ ] Add `man` page generation from clap metadata.
- [ ] Publish the JSON output schema formally if Phase 7 decides it is stable enough.
- [ ] Add a migration guide for users coming from `cargo-msrv` or manual MSRV workflows.
- [ ] Consider a `cargo compatible init` command only if a config file format first proves worthwhile.

Exit criteria:

- [ ] A Rust developer unfamiliar with MSRV tooling can get productive quickly without reading the source.
- [ ] Example outputs and schema docs match the real CLI output.
- [ ] At least one comparison doc explains where `cargo-compatible` fits relative to alternatives.

### Phase 12 - community readiness and 1.0 roadmap

Status: planned

Goal: establish the project as a credible, maintained tool with a clear path to a durable 1.0 contract.

Work to do:

- [ ] Define and document the stability contract for 1.0 (what is SemVer-stable and what is not).
- [ ] Establish a deprecation policy for CLI flags and output schema changes.
- [ ] Set up issue templates and PR templates on GitHub.
- [x] Add a security policy (`SECURITY.md`) with a responsible disclosure process.
- [ ] Decide whether the crate should expose a library API or remain CLI-only through 1.0.
- [ ] Plan and execute early outreach: blog post, forum post, release announcement, or equivalent.
- [ ] Investigate whether deeper Cargo integration (RFC / pre-RFC) is worth pursuing after the CLI stabilizes.
- [ ] Consider privacy-first opt-in telemetry only if there is a concrete product question it would answer.
- [ ] Define the 1.0 feature gate and release cadence explicitly.

Exit criteria:

- [ ] The project has a public 1.0 roadmap with explicit scope boundaries.
- [ ] Security and stability contracts are documented rather than implied.
- [ ] The repo is positioned for outside contribution without maintainers having to explain everything ad hoc.

---

## Verification ledger

These commands are documented in this repository as actually run during the March 20-24, 2026 audit, follow-up, and benchmark passes:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo nextest run`
- `~/.cargo/bin/cargo-deny check`
- `cargo bench --bench large_workspace_resolver --no-run`
- `cargo run -- --help`
- `cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml`
- `cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml`
- `cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml`
- `cargo run -- scan --manifest-path tests/fixtures/missing-rust-version/Cargo.toml --package helper`
- `cargo run -- explain definitely-not-a-package --manifest-path tests/fixtures/path-too-new/Cargo.toml`
- `cargo run -- resolve --manifest-path tests/fixtures/virtual-workspace/Cargo.toml --workspace --format markdown`
- `cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml --format markdown`
- `cargo run -- scan --manifest-path tests/fixtures/mixed-workspace/Cargo.toml --package tests/fixtures/mixed-workspace/members/high/Cargo.toml --format json`
- `cargo run -- scan --manifest-path tests/fixtures/mixed-workspace/Cargo.toml --package members` (expected failure)
- `cargo run -- explain low --manifest-path tests/fixtures/mixed-workspace/Cargo.toml --package high` (expected failure)
- `tmpdir=$(mktemp -d) && cargo run -- resolve --manifest-path tests/fixtures/virtual-workspace/Cargo.toml --workspace --format markdown --write-report "$tmpdir/report.md"`
- `cargo test apply_manifest_suggestions_`
- `cargo test apply_lock_`
- `cargo bench --bench large_workspace_resolver` (all seven benchmark groups: resolve_end_to_end, scan_analysis, metadata_load, temp_workspace_copy, dense_graph_scan, mixed_version_scan, fixture_scan)

---

## Risks

- Some human-facing resolve and explain summaries still do not fully disambiguate same-name crates across all multi-source or same-version cases; package IDs remain the conservative fallback.
- The resolution command now uses `cargo update` (without `--workspace`) to allow dependency upgrading; that is the right behavior for the lockfile-improvement workflow, but it is a deliberate semantic choice that should stay well-tested.
- The temp-copy resolution model is measured and justified at current scale (20–25% of resolve cost); it would need re-evaluation for workspaces significantly larger than 96 members or with heavy filesystem artifacts.
- Manifest suggestion quality depends on local sparse-index or local-registry metadata availability and intentionally does not reimplement full Cargo feature resolution.
- Benchmark coverage now includes multiple workspace topologies and fixture-derived scenarios, but still lacks a registry-backed corpus with real crates.io dependencies; performance claims on registry-heavy repositories remain untested.
- The JSON output schema is not yet formally versioned; downstream consumers still risk breakage on shape changes.
- MSRV is declared as 1.89 and enforced in CI, but a local developer can still miss an MSRV regression until CI runs.
- Windows now runs in CI, but there is still little Windows-specific write-path or temp-path diagnostic coverage beyond the generic test matrix.
- The `crates-index` dependency pins to a specific sparse-index protocol version; future crates.io changes could break lookups in ways that look like user error.
- Phase 6 baselines exist, so Phase 7 docs/examples can now honestly reference measured performance characteristics without overstating readiness.

---

## Open questions

These are the live judgment calls that shape the next few phases:

| Question | Phase | Impact |
|----------|-------|--------|
| ~~What benchmark corpus best approximates real operator pain without making the suite flaky or network-dependent?~~ | 6 | **Resolved**: seven benchmark groups cover metadata, scan, copy, resolve, dense graphs, mixed versions, and fixtures; registry-backed corpus deferred until real adoption feedback arrives. |
| ~~Is temp-workspace copy time a release blocker in practice, or only an aesthetic concern until larger repos are measured?~~ | 6 | **Resolved**: measured at 20–25% of resolve time; not a blocker. The safety model is justified. |
| Should JSON output be versioned in v0.2, or explicitly labeled unstable until more CI-facing features land? | 7 | Downstream contract and release messaging |
| How much package-identity disambiguation is enough before always showing source labels by default? | 7 | Human-report readability vs verbosity |
| Should v0.2 ship as crate-only, or does it need binary artifacts on day one to feel credible? | 8 | Release automation scope |
| What is the right exit-code contract for CI usage? (0/1/2? configurable?) | 9 | CI integration contract |
| Should config file support land before or after CI-oriented outputs like SARIF/JUnit? | 9 | Feature ordering and adoption |
| Should the tool expose a library API or stay CLI-only through 1.0? | 12 | API surface commitment |
| How should the tool handle `rust-version` ranges if Cargo eventually supports them? | 10 | Forward compatibility |

---

## Immediate next moves

1. Start Phase 7 by auditing every README command example against the current CLI and fixing or deleting aspirational content.
2. Audit `--help` text for every subcommand so flags, defaults, and safety notes match actual behavior.
3. Decide whether v0.2 is primarily a "measured core" release, a "CI integration" release, or a hybrid — then cut scope accordingly.

---

## Progress log

- 2026-03-24: Completed Phase 6 — performance realism and benchmark expansion. Expanded `large_workspace_resolver` bench harness from one benchmark group (resolve-only, synthetic linear chain) to seven groups: `resolve_end_to_end`, `scan_analysis`, `metadata_load`, `temp_workspace_copy`, `dense_graph_scan`, `mixed_version_scan`, and `fixture_scan`. Added dense-graph and mixed-version workspace generators. Added fixture-derived scan benchmarks using four real test fixtures. Phase-separated timing proves metadata loading (external `cargo metadata`) dominates resolve cost at 50–76 ms; temp-workspace copy is 18–41 ms (20–25% of resolve); scan analysis itself is 1.5–40 ms depending on scale. No memory pressure observed at 96 members. CI performance thresholds deferred due to ~5–10% benchmark variance at 10-sample level. Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` (all pass), `cargo bench --bench large_workspace_resolver` (all seven groups complete). Next: Phase 7 — release polish and operator trust cleanup.
- 2026-03-24: Fixed the active CI failures by making integration fixtures and snapshot sanitization cross-platform, replacing Windows-hostile path serialization in local-registry and git fixture setup, and correcting the repo's declared MSRV from 1.74 to 1.89 to match the current dependency floor (`cargo_metadata 0.22.0`, `cargo-platform 0.3.2`, `home 0.5.12`, and `smol_str 0.3.6` no longer support 1.74). Verified with: `cargo test --locked`, `cargo +1.89.0 check --all-features --locked`, `cargo +1.88.0 check --all-features --locked` (expected failure), `~/go/bin/actionlint .github/workflows/ci.yml`. Next: keep the dependency floor explicit in release notes when upgrading crates that can silently raise MSRV.
- 2026-03-24: Reframed `BUILD.md` as a live execution manual for the post-hardening stretch, shifting the top-line narrative from "Phases 4–5 complete" to a concrete Phase 6–8 kickoff with measurable next work, updated open questions and risks to reflect the actual remaining decisions, and tightened README/AGENTS status wording so the repo no longer reads like a finished-correctness snapshot. Verified with: `rg -n "Current execution window|Phase 6|Phase 7|Status:" BUILD.md README.md AGENTS.md`, `git diff --check`. Next: start Phase 6 with fixture-derived benchmarks and baseline capture.
- 2026-03-22: Completed Phases 4 and 5. Phase 4: threaded workspace root through `ExplainReport` so `render_explain_human` and `render_explain_markdown` use the actual workspace root instead of `Path::new(".")` for path-relative package labels; confirmed dependency-path chain labels are already properly disambiguated via `colliding_base_labels` + `unique_package_label` in `shortest_paths_from_root`. Phase 5: added `lockfile-improvement` fixture with a 3-version local registry (1.1.0 compatible, 1.2.0 incompatible, 1.3.0 compatible); fixed `run_resolution_command` to use `cargo update` without `--workspace` so dependencies actually get updated to newer compatible versions; added integration tests verifying scan reports 1.2.0 as incompatible and resolve upgrades 1.2.0 → 1.3.0 with non-empty `improved_packages` and empty `remaining_blockers`. Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` (50 tests pass), `cargo nextest run` (50 tests pass). All snapshots unchanged.
- 2026-03-22: Closed out Phases 4 and 5. Threaded workspace root through `ExplainReport` so explain rendering uses workspace-relative path labels instead of `Path::new(".")`. Fixed `run_resolution_command` to use `cargo update` without `--workspace` — the `--workspace` flag limited updates to workspace member entries only, preventing dependency upgrades in the temp-copy resolution flow. Added a `lockfile-improvement` fixture with a local registry containing three versions of `compat-demo` (1.1.0 compatible, 1.2.0 incompatible, 1.3.0 compatible) to exercise the true lockfile-only improvement path end to end. Refactored `stage_local_registry_fixture` into a reusable `stage_local_registry_fixture_with_packages` helper. Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` (50 tests pass), `cargo nextest run` (50 pass), `~/.cargo/bin/cargo-deny check`, `cargo bench --bench large_workspace_resolver --no-run`. Next: begin Phase 6 (benchmark expansion) and Phase 7 (release polish).
- 2026-03-22: Added MSRV badge to README, unit tests for `classify_package`/`strongest_status`/`parse_version_display`, integration tests for `apply-lock` no-op, `scan` incompatible reporting (human and JSON), `explain` JSON blocker classification, and `resolve --write-report` with JSON format. Improved panic message in `parse_version_display` to include the invalid value. Corrected BUILD.md risk register to reflect that MSRV is declared and enforced in CI (1.74 job + cross-platform matrix + dogfood gate already present). Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` (48 tests pass). Next: continue identity-disambiguation and lockfile-improvement work.
- 2026-03-22: Restored the fuller `BUILD.md` execution manual after the docs-tightening regression, refreshed `README.md` to stay concise but truthful, and re-synced the public docs with current package-selection, write-report, local-registry, and package-identity behavior. Verified with: `git diff --check`, `cargo run -- --help`. Next: keep the docs aligned while finishing the remaining identity-disambiguation and lockfile-improvement work.
- 2026-03-22: Added source-aware human report labels for resolved, workspace, and path packages, taught manifest suggestions to read a crates.io local-registry replacement from workspace `.cargo/config.toml`, and added deterministic end-to-end `suggest-manifest --write-manifests` coverage via a local-registry fixture. Verified with: `cargo fmt --check`, `cargo test report::tests:: --lib`, `cargo test --test integration_cli suggest_manifest_write_manifests_uses_local_registry_fixture_end_to_end`, `cargo test --test integration_cli explain_path_dep`, `cargo test --test integration_cli scan_mixed_workspace_human_snapshot`. Stop point: a true lockfile-only improvement fixture is still missing because the current `cargo update --workspace` temp-copy flow preserves existing lockfile selections; that needs a deliberate repro or strategy change, not guesswork. Next: extend package-identity disambiguation into dependency-path chains and identify a credible lockfile-only improvement fixture.
- 2026-03-22: Added direct `apply-lock` coverage, direct manifest-write coverage, and failure-path coverage for missing candidate lockfiles plus partial manifest-apply scenarios; also hardened package identity handling by making manifest-suggestion blocker matching source-aware and by omitting ambiguous multi-version same-identity resolve diffs instead of collapsing them incorrectly. Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo nextest run`, `~/.cargo/bin/cargo-deny check`, `cargo bench --bench large_workspace_resolver --no-run`, `cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml`, `cargo run -- resolve --manifest-path tests/fixtures/virtual-workspace/Cargo.toml --workspace --format markdown`. Next: add a deterministic local-registry fixture for end-to-end `--write-manifests` coverage and keep tightening human-facing package identity disambiguation.
- 2026-03-22: Tightened `--package` to exact workspace-member name, package ID, or manifest-path matching, scoped `explain` to the selected dependency graph, made `resolve --write-report` follow `--format`, improved low-risk package identity labeling in resolve and explain, and added direct integration coverage for `--write-report` and `--write-candidate`. Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo nextest run`, `~/.cargo/bin/cargo-deny check`, `cargo bench --bench large_workspace_resolver --no-run`, `cargo run -- scan --manifest-path tests/fixtures/mixed-workspace/Cargo.toml --package tests/fixtures/mixed-workspace/members/high/Cargo.toml --format json`, `cargo run -- scan --manifest-path tests/fixtures/mixed-workspace/Cargo.toml --package members` (expected failure), `cargo run -- explain low --manifest-path tests/fixtures/mixed-workspace/Cargo.toml --package high` (expected failure), `tmpdir=$(mktemp -d) && cargo run -- resolve --manifest-path tests/fixtures/virtual-workspace/Cargo.toml --workspace --format markdown --write-report "$tmpdir/report.md"`. Next: finish the remaining package-identity cleanup and cover the higher-risk mutating paths.
- 2026-03-20: Audited the repository, captured the implemented command surface, source files, fixture coverage, current limitations, and verified local workflow in the original `BUILD.md`. Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo nextest run`, `~/.cargo/bin/cargo-deny check`, `cargo bench --bench large_workspace_resolver --no-run`, `cargo run -- --help`, `cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml`, `cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml`, `cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml`. Next: perform a deeper code review and identify correctness gaps.
- 2026-03-20: Completed a deeper review of the core modules, CLI, tests, fixtures, CI workflow, and repo docs; identified issues around overly permissive `--package`, Markdown fallbacks in `resolve` and `explain`, unnecessary work and silent success in `explain`, crate-name-only reporting collapse, and thin edge-case coverage on mutating flows. Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo nextest run`, plus spot checks of `--package`, `explain`, and `--format markdown` behavior. Next: land the first wave of fixes and expand CLI integration coverage.
- 2026-03-20: Implemented the first review follow-up pass by restricting `--package` to workspace members, tightening invalid `explain` queries, replacing the Markdown fallbacks for `resolve` and `explain`, and adding integration coverage for those paths. Verified with: `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo run -- scan --manifest-path tests/fixtures/missing-rust-version/Cargo.toml --package helper`, `cargo run -- explain definitely-not-a-package --manifest-path tests/fixtures/path-too-new/Cargo.toml`, `cargo run -- resolve --manifest-path tests/fixtures/virtual-workspace/Cargo.toml --workspace --format markdown`, `cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml --format markdown`. Next: revisit remaining source-aware reporting and write-path coverage gaps.
- 2026-03-21: Re-read the full source tree, tests, benchmark, CI workflow, and repo docs; confirmed the March 20 fixes landed cleanly and found four remaining issues: loose substring-based package matching, out-of-scope `explain` success, `--write-report` ignoring `--format`, and thin direct coverage for mutating or file-output flows. Verified with: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo nextest run`, `cargo deny check`, `cargo bench --bench large_workspace_resolver --no-run`, and targeted manual repros for package selection, explain scoping, and `--write-report` behavior. Next: tighten selection and explain-scope semantics, then add write-path coverage.
- 2026-03-21: Rewrote `BUILD.md` into a fuller execution manual aligned with the maintainer's stronger repo-plan style while keeping `cargo-compatible`-specific behavior, verification history, risks, and next moves intact. Verified with: document audit of the prior `BUILD.md`, `README.md`, `AGENTS.md`, and repository layout. Next: use this plan as the source of truth for the next hardening pass.

---

## Decision log

- 2026-03-24: CI performance thresholds deferred; benchmark noise at the 10-sample level (~5–10% variance) would produce false-positive regressions; revisit when a heavier registry-backed corpus is added or when absolute runtime exceeds operator tolerance.
- 2026-03-24: Temp-workspace copy cost measured at 20–25% of total resolve time (18–41 ms at 32–96 members); the safety model is justified by measurement and should be preserved unless workspaces significantly larger than 96 members expose a bottleneck.
- 2026-03-22: `resolve` now runs `cargo update` without `--workspace` in the temp workspace so that dependencies are actually upgraded to newer compatible versions; the previous `cargo update --workspace` only updated workspace member entries and never changed dependency versions, defeating the purpose of the lockfile improvement workflow; the temp-workspace safety model still prevents mutations to the real checkout.
- 2026-03-22: `ExplainReport` now carries `workspace_root` (skipped in JSON serialization) so that explain report rendering uses workspace-relative path labels instead of relying on `Path::new(".")`.
- 2026-03-22: Detailed resolve version changes are omitted when multiple resolved versions share the same package name and source identity; collapsing them into one before or after pair was misleading, so notes should stay conservative rather than fabricate precision.
- 2026-03-22: `suggest-manifest --write-manifests` stages all targeted edits before atomically persisting each manifest; later lookup failures should not leave earlier manifests partially applied; the mutating path must be safer than a naive sequential write loop.
- 2026-03-22: `--package` matches workspace members only by exact package name, package ID, or normalized manifest path; substring path matching was too error-prone and could silently widen scope; selection errors should stay explicit instead of permissive.
- 2026-03-22: `explain` queries are scoped to the selected dependency graph; a package merely existing somewhere in `cargo metadata` is not enough to justify a successful explanation; success should imply relevance to the chosen graph.
- 2026-03-22: `resolve --write-report` follows the selected `--format`; file output should match the rendered stdout contract unless the CLI says otherwise; automation can still request JSON explicitly with `--format json`.
- 2026-03-20: Missing dependency `rust-version` metadata is treated as `unknown`, not compatible; this is safer than optimistic guessing and matches the current analysis model; reports and follow-up logic must preserve that distinction.
- 2026-03-20: `resolve` runs in a temporary workspace copy rather than mutating the real checkout; this favors safety and debuggability over raw speed; future performance work must justify any change explicitly.
- 2026-03-20: `apply-lock` requires an explicit candidate lockfile path and applies it atomically; this keeps the mutating step narrow and intentional; users should never get an implicit lockfile rewrite from a read-only command.
- 2026-03-20: Manifest suggestions stay conservative and use locally available crates.io metadata only, now from either the sparse cache or a crates.io `local-registry` replacement; this prevents fake certainty for uncached, path, or git dependencies; no suggestion is better than a bogus one.
- 2026-03-20: Tracing is opt-in through `RUST_LOG`; this keeps machine-readable and human-readable CLI output stable by default; debug detail should not leak into normal command output.
- 2026-03-20: Path and git dependencies are analysis and explain surfaces, not fake downgrade-target surfaces; the tool can report them as blockers without inventing crates.io-based edits; suggestion logic should keep that boundary.
- 2026-03-21: `cargo-deny` duplicate-version findings remain informational warnings under the current policy; useful signal without blocking the repo on transitive dependency churn; dependency-policy tightening can happen later if it becomes materially valuable.
- 2026-03-21: The active execution focus has shifted from broad feature creation to correctness hardening and confidence-building; the command surface already exists, but edge semantics and write-path trust still need work; prioritize precision and verification over adding new workflow surface area.

---

*Update this log only with things that actually happened.*
