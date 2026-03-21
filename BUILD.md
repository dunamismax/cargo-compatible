# BUILD.md

> **This is the primary operational handoff document for this repository. Keep it current whenever behavior, tooling, docs, workflows, or repository structure change.**

Audited directly from the repository and local command verification on March 20, 2026.

## Current State

`cargo-compatible` is a Cargo subcommand for analyzing whether a workspace's resolved dependency graph is compatible with a chosen Rust version or MSRV, producing a safer candidate lockfile, explaining blockers, and conservatively suggesting direct manifest changes when lockfile-only resolution is not enough.

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
- opt-in `tracing` instrumentation via `RUST_LOG` without changing default CLI output
- `proptest` invariants for semver candidate selection and resolution diff behavior
- a Criterion benchmark for large synthetic workspace resolution
- CI/local verification coverage with `cargo-nextest`, `cargo-deny`, and benchmark compilation

## Key Files

- `Cargo.toml`
  - Crate metadata, dependencies, dev-dependencies, and bench registration
- `src/main.rs`
  - Thin binary entry point plus opt-in tracing subscriber initialization
- `src/lib.rs`
  - Command dispatch and output routing
- `src/cli.rs`
  - Full CLI surface and examples
- `src/model.rs`
  - Shared report and selection model
- `src/metadata.rs`
  - `cargo metadata` integration, package selection, target rust-version resolution, and tracing events
- `src/compat.rs`
  - Current-state compatibility analysis, dependency path capture, and tracing events
- `src/resolution.rs`
  - Candidate lockfile generation, comparison, apply flow, and resolution diff invariants
- `src/temp_workspace.rs`
  - Safe temp copy support for resolution experiments
- `src/index.rs`
  - Sparse-index cache lookup, compatible version choice logic, tracing, and semver invariants
- `src/manifest_edit.rs`
  - Direct dependency inspection and minimal TOML rewriting
- `src/explain.rs`
  - Explain report assembly and blocker classification
- `src/report.rs`
  - Human, JSON, and Markdown rendering
- `tests/integration_cli.rs`
  - Snapshot-backed CLI integration tests
- `tests/version_selection.rs`
  - Example-driven coverage for registry candidate selection rules
- `benches/large_workspace_resolver.rs`
  - Criterion benchmark that generates large path-only workspaces and measures `build_candidate_resolution`
- `deny.toml`
  - Cargo-deny policy for advisories, bans, licenses, and allowed sources
- `tests/fixtures/*`
  - Deterministic sample workspaces for key compatibility scenarios
- `.github/workflows/ci.yml`
  - Enforced `fmt`, `clippy`, `cargo-deny`, `cargo-nextest`, and benchmark compile workflow

## Fixture And Test Coverage

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

Property coverage currently exists for:

- registry candidate selection invariants in `src/index.rs`
- version-diff invariants in `src/resolution.rs`

Benchmark coverage currently exists for:

- synthetic large path-only workspaces with 32 and 96 members in `benches/large_workspace_resolver.rs`

## Verified Build And Run Workflow

All commands below were run directly in `/Users/sawyer/github/cargo-compatible` on March 20, 2026.

| Command | Result | Notes |
| --- | --- | --- |
| `cargo fmt --check` | Success | Formatting clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | Success | No warnings |
| `cargo test` | Success | Unit, property, snapshot, and integration suites passed |
| `cargo nextest run` | Success | 11 tests passed under nextest |
| `~/.cargo/bin/cargo-deny check` | Success | Advisories, bans, licenses, and sources passed; duplicate-version warnings remain informational |
| `cargo bench --bench large_workspace_resolver --no-run` | Success | Bench harness compiles in optimized mode |
| `cargo run -- --help` | Success | Confirms direct CLI/help surface |
| `cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml` | Success | Confirms current-state reporting |
| `cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml` | Success | Confirms blocker/path explanation |
| `cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml` | Success | Confirms dry-run suggestion path |

## Config And Runtime Notes

- No environment variables are required for normal `scan`, `resolve`, `apply-lock`, or `explain` operation.
- `RUST_LOG` is optional and enables `tracing` output for metadata loading, analysis, registry selection, and temporary resolution flow.
- `cargo metadata` must succeed for the selected workspace.
- `resolve` uses a full temporary copy of the workspace and runs stable Cargo commands there.
- `suggest-manifest` only uses crates.io sparse index entries already present in the local Cargo cache.
- Missing package `rust-version` metadata is surfaced as unknown compatibility, not success.
- `apply-lock` fails safely when the requested candidate lockfile does not exist.
- CI installs `cargo-deny` and `cargo-nextest` explicitly before running checks.
- `cargo-deny` currently reports duplicate-version warnings from transitive dependencies; the policy keeps these at warning level.
- The Criterion benchmark generates large local path-only workspaces to measure resolver overhead without network activity.

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
  - candidate lockfile generation, apply semantics, and resolution diff invariants
- `src/index.rs`
  - semver candidate selection semantics and property-tested invariants
- `src/manifest_edit.rs`
  - manifest suggestion heuristics and write behavior
- `src/report.rs`
  - output format content and structure
- `benches/large_workspace_resolver.rs`
  - benchmark scope and synthetic large-workspace shape
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
   - The benchmark tracks this cost, but does not change the temp-copy design.
4. Mixed-workspace guidance is explanatory.
   - The tool reports resolver guidance and unification blockers.
   - It does not auto-edit workspace resolver settings in v1.
5. Path and git dependencies are analysis-only for suggestions.
   - They can be reported as blockers and included in explanations.
   - They do not get fake crates.io downgrade recommendations.
6. Benchmark coverage is intentionally synthetic.
   - `benches/large_workspace_resolver.rs` models large local workspaces with path dependencies.
   - It does not yet represent registry-heavy or feature-heavy real-world repos.

## Code Review Findings — 2026-03-20

Review scope for this pass:

- read the core source modules, CLI/tests/fixtures, CI workflow, and repo docs
- re-ran `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings`
- re-ran `cargo test` and `cargo nextest run`
- spot-checked CLI behavior for `--package`, `explain`, and `--format markdown`

### Strengths

- The architecture is cleanly segmented by responsibility: CLI parsing, metadata loading, compatibility analysis, candidate resolution, manifest editing, explain flow, and report rendering are separated and easy to navigate.
- The safety model is strong for a v0.1 tool: `resolve` works in a temp workspace copy, `apply-lock` is explicit, lockfile writes are atomic, and manifest suggestions default to dry-run.
- Repo hygiene is solid: CI enforces `fmt`, `clippy`, `cargo-deny`, `cargo-nextest`, and bench compilation, while `README.md`, `AGENTS.md`, and this file are unusually well aligned.

### Findings / Risks

1. `--package` is currently too permissive.
   - `src/metadata.rs` matches against all `cargo metadata` packages, not just workspace members.
   - In practice, `cargo run -- scan --package anyhow --format json` succeeds in this repo and treats the transitive registry crate `anyhow` as the selected package.
   - That breaks the expected meaning of package selection and can produce misleading target selection and dependency-path analysis.
2. Markdown output is incomplete for two commands.
   - `src/report.rs` implements real Markdown for `scan` and `suggest-manifest`, but `render_resolve_markdown` and `render_explain_markdown` currently return pretty JSON.
   - Spot checks confirmed `cargo compatible resolve --format markdown` and `cargo compatible explain --format markdown` emit JSON, so docs and behavior diverge.
3. `explain` does unnecessary work and hides lookup failures.
   - `src/explain.rs` always builds a full candidate resolution, even when the query does not resolve to any package.
   - `cargo run -- explain definitely-not-a-package --manifest-path tests/fixtures/path-too-new/Cargo.toml` exits successfully and only prints the query instead of a clear error.
   - This is both a performance issue and a developer-ergonomics issue.
4. Some reporting logic collapses distinct packages by crate name only.
   - `src/resolution.rs` derives `improved_packages` and `remaining_blockers` from package names, and `src/explain.rs` falls back to package-name matching when inferring a candidate version.
   - Workspaces that contain the same crate name from multiple sources or multiple concurrently resolved versions could get misleading summaries.
5. Coverage is strongest on happy-path scan/resolve behavior, but thin on edge and mutating flows.
   - There is no direct test coverage for `apply-lock`, `--write-candidate`, `--write-report`, `--write-manifests`, invalid `explain` queries, markdown output, or rejecting non-workspace `--package` selections.
   - Those are the most likely regression points because they sit on I/O and UX boundaries.

### Recommended Next Steps

1. Restrict `--package` matching to workspace members and fail fast on transitive/non-member specs.
2. Implement actual Markdown renderers for `resolve` and `explain`, then snapshot-test them.
3. Make `explain` validate the query up front, return a non-zero error on no match, and avoid full candidate resolution when unnecessary.
4. Add focused tests for write/apply flows and duplicate-name or multi-source reporting cases.

## Implementation Follow-Up — 2026-03-20

Implemented against the March 20 review findings:

- Restricted `--package` matching to workspace members only in `src/metadata.rs`.
  - Non-member and transitive package specs now fail fast instead of being treated as selected workspace packages.
- Tightened `explain` query handling in `src/explain.rs`.
  - Unknown queries now return a non-zero error with a clear message.
  - The command now skips candidate lockfile resolution when the queried package is already compatible and no blocker analysis is needed.
- Replaced the Markdown fallbacks in `src/report.rs` with real Markdown renderers for `resolve` and `explain`.
- Expanded CLI integration coverage in `tests/integration_cli.rs`.
  - Added tests for non-workspace `--package` rejection, invalid `explain` queries, and Markdown snapshots for `resolve` and `explain`.

Verification run after implementation:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- spot checks:
  - `cargo run -- scan --manifest-path tests/fixtures/missing-rust-version/Cargo.toml --package helper` -> exits non-zero with workspace-member error
  - `cargo run -- explain definitely-not-a-package --manifest-path tests/fixtures/path-too-new/Cargo.toml` -> exits non-zero with resolved-package error
  - `cargo run -- resolve --manifest-path tests/fixtures/virtual-workspace/Cargo.toml --workspace --format markdown`
  - `cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml --format markdown`

Remaining notable follow-ups:

- Reporting still collapses some resolve/explain summaries by crate name rather than fully source-aware identity.
- Edge and mutating flows remain lightly covered compared with scan/resolve happy paths, especially `apply-lock`, `--write-candidate`, `--write-report`, and `--write-manifests`.
- Ambiguous `explain` queries that match duplicate crate names across sources or versions are still not disambiguated beyond the existing query matching behavior.

## Next Pass Priorities

1. Add more fixture workspaces for real lockfile-only improvement cases and direct dependency semver blockers.
2. Expand the Criterion coverage to model registry-heavy and feature-heavy large workspaces.
3. Improve source/registry-aware manifest suggestion coverage for renamed and target-specific dependencies.
4. Tighten explain output for feature-restriction and mixed-workspace blocker distinctions.
5. Expand fixture coverage around path and git dependency reporting.

## Next-Agent Checklist

1. Read `BUILD.md` first.
2. Then read:
   - `src/cli.rs`
   - `src/lib.rs`
   - `src/metadata.rs`
   - `src/compat.rs`
   - `src/resolution.rs`
   - `src/index.rs`
   - `src/manifest_edit.rs`
   - `src/report.rs`
   - `tests/integration_cli.rs`
3. Run:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test`
   - `cargo nextest run`
   - `cargo-deny check` or `~/.cargo/bin/cargo-deny check`
   - `cargo bench --bench large_workspace_resolver --no-run`
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
   - `CHANGELOG.md`
   - fixtures and snapshots as needed

## Code Review Addendum — 2026-03-21

Fresh review scope for this pass:

- re-read the full source tree, tests, benchmark, CI workflow, and repo docs
- re-ran `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo nextest run`, `cargo deny check`, and `cargo bench --bench large_workspace_resolver --no-run`
- ran targeted manual repros for package selection, explain scoping, and `--write-report` behavior

### Current strengths

- The March 20 follow-up fixes landed cleanly: non-workspace `--package` selection now fails, invalid `explain` queries fail fast, and `resolve`/`explain` Markdown output is now real Markdown instead of JSON fallback.
- The architecture still reads well: metadata loading, graph analysis, candidate resolution, manifest editing, explain flow, and reporting remain cleanly separated and easy to audit.
- CI and local quality gates are in good shape for a young CLI tool. This pass was green on `fmt`, `clippy -D warnings`, `test`, `nextest`, `cargo-deny`, and bench compilation.
- Repo docs are still unusually disciplined overall: `README.md`, `AGENTS.md`, and the existing BUILD notes match the high-level behavior and safety model.

### Remaining issues / risks

1. `--package` matching is still too loose for short or common specs.
   - `src/metadata.rs` still treats `package.manifest_path.contains(spec)` as a valid match.
   - In a temporary two-member workspace (`a`, `b`), `cargo compatible scan --package a` selected both members because both manifest paths contained the substring `a`.
   - That can silently widen scope, flip target selection to `mixed-or-missing`, and make downstream reports look wrong even when the requested package name is exact.
2. `explain` is not scoped to the selected dependency graph.
   - `src/metadata.rs::package_id_from_query` searches all resolved packages in metadata, not only packages reachable from the selected members.
   - Repro: selecting only member `a` by exact manifest path still allows `cargo compatible explain b ...` to succeed and report `b` as `compatible` with no dependency path, even though `b` is outside the selected analysis scope.
   - That is a CLI UX and correctness problem: success currently means “package exists somewhere in metadata,” not “package is present in the selected graph.”
3. `--write-report` ignores `--format`.
   - `src/lib.rs` always writes `ResolveReport` as pretty JSON when `write_report` is set, regardless of `--format`.
   - Repro: `cargo compatible resolve --format markdown --write-report /tmp/report.md` prints Markdown to stdout but writes JSON to the file.
   - That is a small but real automation footgun and the current docs do not make the behavior explicit.
4. Test coverage still has blind spots around edge and mutating flows.
   - Current integration coverage is good for fixture-backed scan/resolve/explain behavior, but it does not assert exact positive `--package` selection, out-of-scope `explain` queries, or `--write-report` format semantics.
   - `apply-lock`, `--write-candidate`, and `--write-manifests` are still not exercised directly in integration tests.
   - The selection/scoping issues above slipped through precisely because those boundaries are not yet pinned down by tests.

### Recommended next steps

1. Replace manifest-path substring matching with exact package-name / package-id / normalized manifest-path matching, then add focused tests for short package names and ambiguous path fragments.
2. Make `explain` require reachability from the selected members and fail clearly when a query resolves to a package outside the selected graph.
3. Either make `--write-report` honor `--format` or document/rename it as JSON-only, then test stdout-vs-file behavior explicitly.
4. Add temp-dir integration coverage for `apply-lock`, `--write-candidate`, and `--write-manifests` so the I/O boundary is verified, not assumed.
