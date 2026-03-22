# BUILD.md

Short operator notes for cargo-compatible.

## Commands

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo nextest run
cargo bench --bench large_workspace_resolver --no-run
cargo run -- --help
cargo run -- scan --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- resolve --manifest-path tests/fixtures/virtual-workspace/Cargo.toml --workspace --format markdown
cargo run -- explain too_new --manifest-path tests/fixtures/path-too-new/Cargo.toml
cargo run -- suggest-manifest --manifest-path tests/fixtures/path-too-new/Cargo.toml
```

## Notes

- `scan` and `resolve` are dry-run-first workflows
- `resolve` uses a temporary workspace copy instead of mutating the real checkout
- keep fixture workspaces and output snapshots aligned with command behavior
