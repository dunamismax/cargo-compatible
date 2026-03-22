# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Local-registry-backed manifest suggestion lookup plus deterministic end-to-end `suggest-manifest --write-manifests` coverage for a crates.io source-replacement fixture.
- Temp-dir git-fixture coverage for same-name dependency-path disambiguation in CLI output.
- Clearer human-facing package identity labels for workspace/path packages and resolve version changes so same-name crates are easier to distinguish in reports.
- Direct `apply-lock` coverage, direct manifest-write coverage, and failure-path coverage for missing candidate lockfiles and partial manifest-apply scenarios.
- Opt-in `tracing` initialization and instrumentation around metadata loading, workspace analysis, registry candidate selection, and temporary resolution runs.
- `cargo-deny` policy configuration in `deny.toml` and CI coverage for dependency-policy checks.
- `proptest` invariants for semver candidate selection and resolution diff behavior.
- Criterion benchmark for large synthetic workspaces that exercises `build_candidate_resolution`.
- Cross-platform CI matrix (Linux, macOS, Windows).
- MSRV verification in CI (Rust 1.74).
- Dogfood CI job that runs the tool against its own workspace.
- Release workflow with binary builds for 5 targets and crates.io publish.
- `CONTRIBUTING.md` with development setup, coding standards, and PR process.
- `SECURITY.md` with vulnerability reporting process.
- `.editorconfig` for editor consistency.
- GitHub issue and PR templates.

### Changed

- Tightened `--package` to use exact workspace member name/package-ID/manifest-path matching instead of manifest-path substring matches.
- Made `explain` reject queries outside the selected dependency graph and clarified ambiguous-query guidance.
- Made `resolve --write-report` write the same rendered format selected by `--format`.
- Hardened package identity handling by avoiding collapsed multi-version resolve diffs for the same package identity and by matching manifest suggestions on package name plus source instead of name alone.
- Extended package-identity disambiguation into dependency-path chains, stabilized resolve diff identities across temp-workspace copies for same-name path/source cases.
- Switched CI test execution to `cargo-nextest`.
- Declared MSRV of 1.74 in `Cargo.toml`.

### Fixed

- Package selection no longer silently widens the analysis scope via substring matching.
- `explain` no longer succeeds for packages outside the selected dependency graph.
- Multi-version same-identity resolve diffs are no longer collapsed into misleading single change lines.

### Documentation

- Documented that landing a true lockfile-only improvement fixture needs a different Cargo invocation strategy because stable `cargo update --workspace` preserves valid existing lockfile selections.
- Expanded `BUILD.md` with phases 8-12 covering CI/CD hardening, ecosystem integration, advanced analysis, documentation, and 1.0 roadmap.
- Overhauled `README.md` with badges, comparison table, and improved onboarding flow.

## [0.1.0] - 2026-03-19

### Added

- Initial release.
- `scan` command for analyzing current workspace dependency graph compatibility.
- `resolve` command for building candidate lockfiles in sandboxed temp workspaces.
- `apply-lock` command for atomically applying saved candidate lockfiles.
- `suggest-manifest` command for conservative direct dependency requirement suggestions.
- `explain` command for per-package compatibility explanations with dependency paths.
- Workspace-aware Rust version selection and mixed-member analysis.
- Human, JSON, and Markdown output formats.
- Conservative manifest suggestion support backed by the local crates.io sparse index cache.
- CLI integration tests and snapshot coverage.
- CI pipeline with fmt, clippy, nextest, and cargo-deny.

[Unreleased]: https://github.com/dunamismax/cargo-compatible/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dunamismax/cargo-compatible/releases/tag/v0.1.0
