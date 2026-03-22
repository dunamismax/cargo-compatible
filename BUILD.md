# BUILD.md

## Purpose

This file is the canonical execution and tracking document for `cargo-compatible`.

It keeps the repo honest while the project moves from correctness hardening toward release polish and ecosystem integration. At any point it should answer:

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

## Long-Term Vision

`cargo-compatible` aims to become the standard Cargo companion for MSRV management across the Rust ecosystem. The trajectory:

1. **v0.1 — Correctness foundation** (current): prove the analysis engine is trustworthy on real workspaces.
2. **v0.2 — Ecosystem integration**: CI-native output, GitHub Actions, `cargo-deny` interop, lockfile-only downgrade.
3. **v0.3 — Intelligence**: smarter resolution strategies, feature-aware analysis, workspace-level policy.
4. **v0.4 — Performance**: incremental analysis, caching, parallel resolution.
5. **v1.0 — Production-grade**: stable CLI contract, SemVer output schema, exhaustive edge-case coverage.

The non-goal boundary is equally important: this tool should never become an alternative package manager, a build system, or a CI orchestrator. It reads Cargo metadata and lockfiles, experiments in sandboxes, and reports findings. It does not own the build.

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

## Repo snapshot

**Current phase: v0.1 — correctness hardening (Phases 4–5 active)**

What exists:
- Full command surface: `scan`, `resolve`, `apply-lock`, `suggest-manifest`, `explain`
- Fixture-backed integration tests and snapshot coverage
- CI gate with fmt, clippy, test, nextest, cargo-deny, and benchmark compilation
- Human, JSON, and Markdown output modes
- Published on crates.io as `cargo-compatible` 0.1.0

What is actively being hardened:
- Package-identity disambiguation in dependency-path chains
- Write-path and mutating-flow coverage
- True lockfile-only improvement fixture

What does **not** exist yet:
- Formal JSON output schema versioning
- Windows CI matrix
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
  - deterministic sample workspaces for missing rust-version, mixed-workspace, path dependency, local-registry manifest-blocker, and resolver-guidance cases
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

This repository is no longer in a greenfield feature-planning phase. The current work is correctness hardening and operator trust building around:

- broader confidence in the remaining mutating and file-writing flows
- deeper same-name and multi-source package-identity disambiguation in human-facing output
- more realistic performance evidence beyond the current synthetic benchmark
- release-polish passes that keep the public docs as accurate as the code and snapshots
- CI hardening and ecosystem integration for real-world adoption

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

### Completed phases

- Phase 0 - repo charter, source-of-truth mapping, and verification baseline. Status: **done**.
- Phase 1 - core command surface and analysis engine. Status: **done**.
- Phase 2 - safe resolution and manifest-suggestion workflow. Status: **done**.
- Phase 3 - reporting, fixtures, CI, and benchmark baseline. Status: **done**.

### Active phases

- Phase 4 - correctness hardening for package selection, explain scope, and report semantics. Status: **in progress**.
- Phase 5 - write-path and mutating-flow coverage. Status: **in progress**.

### Planned phases

- Phase 6 - performance realism and benchmark expansion. Status: **not started**.
- Phase 7 - release polish and operator trust cleanup. Status: **not started**.
- Phase 8 - CI/CD hardening and release automation. Status: **not started**.
- Phase 9 - ecosystem integration and interoperability. Status: **not started**.
- Phase 10 - advanced analysis and resolution intelligence. Status: **not started**.
- Phase 11 - documentation, examples, and onboarding. Status: **not started**.
- Phase 12 - community readiness and 1.0 roadmap. Status: **not started**.

---

## Active phase detail

### Phase 4 - correctness hardening for package selection, explain scope, and report semantics

Status: in progress

- [x] Replace loose manifest-path substring matching with exact package-name, package-id, or normalized manifest-path matching.
- [x] Make `explain` require reachability from the selected dependency graph instead of merely existing somewhere in metadata.
- [x] Make `resolve --write-report` honor `--format` and pin it down with tests.
- [x] Add focused tests for short package names, ambiguous path fragments, and out-of-scope `explain` queries.
- [x] Stop collapsing ambiguous multi-version same-identity resolve diffs into misleading single before or after pairs.
- [x] Make manifest-suggestion blocker matching source-aware so same-name crates from other sources do not trigger bogus rewrite suggestions.
- [ ] Extend the remaining package-identity cleanup into dependency-path chains and the harder same-name multi-source cases that still need package-ID fallback.

Exit criteria:

- [x] Package selection can no longer silently widen the analysis scope.
- [x] `explain` success means the queried package is actually in the selected graph.
- [x] File-report behavior is explicit, tested, and unsurprising.

### Phase 5 - write-path and mutating-flow coverage

Status: in progress

- [x] Add temp-dir integration coverage for `apply-lock`.
- [x] Add direct coverage for `--write-candidate`.
- [x] Add direct coverage for `--write-report`.
- [x] Add direct file-write coverage for `--write-manifests`.
- [x] Verify failure behavior for missing candidate lockfiles and partial-write scenarios where practical.
- [x] Add a deterministic local-registry fixture that exercises direct manifest blocker cases end to end.
- [ ] Add a fixture scenario that exercises a true lockfile-only improvement.

Exit criteria:

- [ ] The highest-risk write paths are verified by tests or explicitly recorded manual proof, not assumed from code reading.

---

## Planned phase detail

### Phase 6 - performance realism and benchmark expansion

Status: not started

Goal: prove the tool performs acceptably on real-world workspace sizes and identify optimization targets before they become release blockers.

- [ ] Expand the Criterion surface beyond the current path-only synthetic benchmark.
- [ ] Add scenarios that better approximate registry-heavy or feature-heavy workspaces.
- [ ] Measure whether the temp-copy resolution model needs targeted optimization while preserving its safety properties.
- [ ] Add benchmarks for the analysis pass itself (graph walking, dependency-path computation).
- [ ] Profile memory usage on large workspaces (500+ resolved packages).
- [ ] Record meaningful resolver-performance conclusions here instead of relying on memory.
- [ ] Establish performance regression thresholds in CI (fail if scan takes >Xms on the synthetic workspace).

Exit criteria:

- [ ] At least two benchmark scenarios beyond the current synthetic one.
- [ ] Documented performance characteristics for workspaces of various sizes.
- [ ] Any optimization decisions are driven by measurement, not guessing.

### Phase 7 - release polish and operator trust cleanup

Status: not started

Goal: make the 0.2 release artifact trustworthy, well-documented, and unsurprising for first-time users.

- [ ] Refresh `README.md`, `CHANGELOG.md`, and `BUILD.md` after the current hardening phases land.
- [ ] Ensure output-file semantics and CLI examples match the actual shipped behavior.
- [ ] Revisit remaining known limitations and classify them as fixed, intentionally deferred, or release-blocking.
- [ ] Tighten any remaining doc wording that overstates coverage or confidence.
- [ ] Verify `--help` text for every subcommand against actual behavior.
- [ ] Ensure all error messages are actionable (tell the user what to do, not just what failed).
- [ ] Add shell completion generation (`clap_complete`).
- [ ] Review and finalize the JSON output schema for stability guarantees.

Exit criteria:

- [ ] A new user can install, run `cargo compatible scan --workspace`, and understand the output without reading BUILD.md.
- [ ] JSON output schema is documented and versioned.
- [ ] All CLI examples in README actually work as shown.

### Phase 8 - CI/CD hardening and release automation

Status: not started

Goal: make the release process repeatable, safe, and confidence-inspiring for both maintainers and users.

- [x] Add MSRV verification to CI (test against the declared `rust-version` in Cargo.toml).
- [x] Add cross-platform CI matrix (Linux, macOS, Windows).
- [ ] Add a release workflow that publishes to crates.io on tag push with changelog validation.
- [ ] Add binary release builds for common platforms (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64) via `cargo-dist` or manual cross-compilation.
- [ ] Add `cargo-audit` to CI for security advisory checks beyond `cargo-deny`.
- [ ] Add integration test coverage that runs against a real published crate's workspace (smoke test against a known-good external project).
- [x] Add CI job that runs the tool against its own workspace as a dogfood gate.
- [ ] Consider `cargo-semver-checks` to prevent accidental breaking changes to the public API.
- [ ] Add dependabot or renovate for automated dependency updates.

Exit criteria:

- [ ] Tagging a release on GitHub produces crates.io publish + binary artifacts automatically.
- [ ] CI catches MSRV regressions before they ship.
- [ ] The tool successfully analyzes itself in CI.

### Phase 9 - ecosystem integration and interoperability

Status: not started

Goal: make `cargo-compatible` a natural part of Rust development workflows rather than a standalone tool.

- [ ] Add `--exit-code` mode for CI usage (exit 1 if incompatible packages found, exit 0 if clean).
- [ ] Add GitHub Actions integration (official action or documented workflow snippet).
- [ ] Add SARIF or SARIF-like output for GitHub Code Scanning integration.
- [ ] Add `cargo-deny` interop: ability to consume `deny.toml` policies or produce compatible output.
- [ ] Add `--diff` mode that compares two lockfiles and reports compatibility changes (useful for PR review).
- [ ] Investigate `rust-version` field adoption across the crates.io ecosystem to quantify the "unknown" problem and guide users.
- [ ] Add support for reading `.clippy.toml` or a `.cargo-compatible.toml` config file for project-level defaults.
- [ ] Add JUnit XML output for CI systems that consume test results.
- [ ] Consider LSP or editor integration for inline MSRV warnings.

Exit criteria:

- [ ] The tool can be dropped into a CI pipeline with a one-liner and fail the build on regressions.
- [ ] At least one CI output format (SARIF, JUnit, or exit code) is production-ready.
- [ ] Project-level configuration reduces per-invocation flag noise.

### Phase 10 - advanced analysis and resolution intelligence

Status: not started

Goal: make the analysis engine smarter without sacrificing its conservative safety properties.

- [ ] Implement feature-aware compatibility analysis (a crate may be compatible when specific features are disabled).
- [ ] Add `--ignore` flag to exclude specific crates from analysis (known false positives).
- [ ] Add `--policy` flag for workspace-level MSRV policies ("all members must support 1.70+").
- [ ] Investigate using `cargo tree` output as an alternative/complement to `cargo metadata` for richer dependency information.
- [ ] Add transitive dependency pinning suggestions when a lockfile-only fix exists but requires pinning a transitive dep.
- [ ] Add workspace-aware `rust-version` propagation suggestions (if member A requires 1.75 but depends on member B at 1.70, suggest alignment).
- [ ] Investigate incremental analysis: if only `Cargo.lock` changed, skip the full metadata reload.
- [ ] Add `--upgrade-path` mode that shows the minimum set of changes to reach a target Rust version.
- [ ] Consider `--simulate` mode that predicts the effect of a Rust version bump without actually running `cargo update`.

Exit criteria:

- [ ] Feature-aware analysis reduces false positives by a measurable amount on real workspaces.
- [ ] At least one "smart suggestion" mode goes beyond the current conservative direct-dependency-only approach.
- [ ] All new analysis modes maintain the safety invariant (never silently mutate, never fabricate certainty).

### Phase 11 - documentation, examples, and onboarding

Status: not started

Goal: make the tool accessible to users who are not Cargo internals experts.

- [ ] Write a user guide (mdbook or standalone markdown) covering common workflows end-to-end.
- [ ] Add annotated example outputs for each command showing what each section means.
- [ ] Add a "troubleshooting" section covering common failure modes and their resolutions.
- [ ] Add a "concepts" page explaining MSRV, `rust-version`, resolver behavior, and why lockfile-first matters.
- [ ] Create example fixtures that users can clone and run the tool against for learning.
- [ ] Add `man` page generation from clap metadata.
- [ ] Document the JSON output schema formally (JSON Schema or similar).
- [ ] Add a "migration guide" for users coming from `cargo-msrv` or manual MSRV management.
- [ ] Consider a `cargo compatible init` command that scaffolds `.cargo-compatible.toml` with sane defaults.

Exit criteria:

- [ ] A Rust developer unfamiliar with MSRV tooling can go from zero to productive in under 10 minutes.
- [ ] JSON schema is published and versioned.
- [ ] At least one comparison document positions `cargo-compatible` against alternatives.

### Phase 12 - community readiness and 1.0 roadmap

Status: not started

Goal: establish the project as a credible, maintained, community-ready tool that users and organizations can depend on.

- [ ] Define and document the stability contract for 1.0 (what is SemVer-stable, what is not).
- [ ] Establish a deprecation policy for CLI flags and output schema changes.
- [ ] Set up issue templates and PR templates on GitHub.
- [ ] Add a security policy (`SECURITY.md`) with responsible disclosure process.
- [ ] Evaluate whether the crate should expose a library API or remain CLI-only.
- [ ] Plan and execute outreach: blog post, Rust users forum announcement, r/rust post.
- [ ] Investigate integration with `cargo` itself (RFC or pre-RFC for MSRV-aware resolution).
- [ ] Add usage telemetry opt-in for understanding real-world usage patterns (privacy-first, disabled by default).
- [ ] Consider organizational sponsorship or Rust Foundation alignment.
- [ ] Define the 1.0 feature gate: what must be done before the major version bump.
- [ ] Establish a release cadence (monthly? per-milestone? ad hoc?).

Exit criteria:

- [ ] The project has a clear, public 1.0 roadmap.
- [ ] Security and stability contracts are documented.
- [ ] The project is positioned for community contribution and long-term maintenance.

---

## Currently verified commands

These commands are documented in this repository as actually run during the March 20-22, 2026 audit and follow-up passes:

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

---

## Open questions

These questions need answers before or during the indicated phase:

| Question | Phase | Impact |
|----------|-------|--------|
| How far should `explain` query matching go beyond exact package ID, name, or `name@version`? | 4 | Edge-case UX for ambiguous queries |
| How source-aware should resolve/explain reporting become for same-name multi-source crates? | 4–5 | Human-facing output clarity |
| Should mixed-workspace resolver guidance remain explanatory only for v0.1? | 7 | Scope of edit automation |
| Should the tool expose a library API or stay CLI-only through 1.0? | 12 | API surface commitment |
| What is the right exit-code contract for CI usage? (0/1/2?) | 9 | CI integration contract |
| Should config file support land before or after JSON schema stabilization? | 9 | Feature ordering |
| How should the tool handle `rust-version` ranges if/when Cargo supports them? | 10 | Forward compatibility |

---

## Risk register

- Some human-facing resolve and explain summaries still do not fully disambiguate same-name crates across all multi-source or same-version cases; package IDs remain the escape hatch.
- A true lockfile-only improvement fixture is still missing; the current temp-copy `cargo update --workspace` strategy preserves existing lockfile choices, so a nontrivial improvement scenario needs a deliberate repro or an explicitly deferred strategy change.
- The temp-copy resolution model is intentionally safe, but it may become a performance pain point on larger real-world workspaces if it is not measured carefully.
- Manifest suggestion quality depends on local sparse-index or local-registry metadata availability and intentionally does not reimplement full Cargo feature resolution.
- The current benchmark is useful but synthetic; it does not yet prove behavior on registry-heavy or feature-heavy repositories.
- The JSON output schema is not yet formally versioned; downstream consumers risk breakage on shape changes.
- MSRV is declared as 1.74 in Cargo.toml and enforced in CI; any dependency bump that silently raises the floor would still only be caught on the MSRV CI job, not locally.
- Windows compatibility is untested; path handling and temp-workspace logic may have platform-specific bugs.
- The `crates-index` dependency pins to a specific sparse-index protocol version; future crates.io changes could break lookups silently.

---

## Immediate next moves

1. Extend package-identity disambiguation into dependency-path chains and the harder same-name multi-source cases that still fall back to package IDs.
2. Add or explicitly defer a fixture that demonstrates a true lockfile-only improvement so the write-path and reporting flow is exercised against a less trivial resolution change.
3. Keep the docs, snapshots, and source-of-truth sections aligned as the remaining hardening work lands.
4. Begin planning the v0.2 release scope.

---

## Progress log

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
