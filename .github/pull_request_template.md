## What

Brief description of the change.

## Why

What problem does this solve or what feature does it add?

## How

Key implementation details, if not obvious from the diff.

## Verification

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo nextest run` passes
- [ ] Snapshots updated if output changed (`cargo insta review`)
- [ ] Docs updated if behavior changed (README, AGENTS.md, BUILD.md)
