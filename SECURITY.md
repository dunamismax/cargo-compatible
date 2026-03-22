# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a vulnerability

If you discover a security vulnerability in `cargo-compatible`, please report it responsibly.

**Do not file a public GitHub issue for security vulnerabilities.**

Instead, email **security@dunamismax.com** with:

- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)

You should receive a response within 48 hours acknowledging receipt. We will work with you to understand the issue and coordinate a fix and disclosure timeline.

## Scope

`cargo-compatible` is a read-mostly tool that:

- Reads `Cargo.toml`, `Cargo.lock`, and `cargo metadata` output
- Creates temporary workspace copies for sandboxed resolution
- Optionally writes candidate lockfiles and manifest edits to user-specified paths
- Queries the local crates.io sparse-index cache

Security concerns specific to this tool include:

- **Path traversal**: ensuring file writes stay within expected directories
- **Temp directory cleanup**: ensuring temporary workspaces are removed
- **Dependency confusion**: ensuring registry lookups use the expected source
- **Command injection**: ensuring no user input reaches shell execution unsanitized

## Dependencies

We use `cargo-deny` to audit dependencies for known security advisories. This runs in CI on every push and PR.
