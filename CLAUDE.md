# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About

hemli is a secret management CLI tool for local development. It caches secrets in the OS-native keyring and fetches them on-demand from external providers via shell commands. Written in Rust (edition 2024).

## Build & Development Commands

This project uses `mise` for task orchestration and `cargo nextest` as the test runner.

- **Build:** `cargo build`
- **Format:** `cargo fmt --all` (nightly variant: `cargo +nightly fmt --all`)
- **Lint:** `cargo clippy --all-targets --all-features -- -D warnings`
- **Test (all):** `cargo nextest run --all-targets`
- **Test (single):** `cargo nextest run -E 'test(test_name)'`
- **Test (integration, requires OS keyring):** `cargo nextest run -- --ignored`
- **Full CI pipeline:** `mise run ci` (runs format check, clippy, test, build in order)

## Architecture

The codebase follows a three-layer design:

**CLI Layer** (`src/cli.rs`) — Clap-based argument parsing. Five subcommands: `get`, `delete`, `list`, `inspect`, `edit`.

**Command Handlers** (`src/main.rs`) — Business logic for each subcommand:
- `cmd_get`: Check keyring cache → if expired/missing, fetch from source command → store in keyring + update index → print to stdout
- `cmd_delete`: Remove from keyring and index
- `cmd_list`: Query index, optionally filter by namespace
- `cmd_inspect`: Read cached secret from keyring and print full metadata as JSON
- `cmd_edit`: Modify metadata (TTL, source command) of a cached secret without re-fetching

**Storage & Integration Layer:**
- `src/store.rs` — Keyring operations. Service name convention: `hemli:<namespace>`. Secrets stored as JSON with metadata.
- `src/source.rs` — Executes external commands via `sh -c` (SourceType::Sh) or direct execution (SourceType::Cmd).
- `src/index.rs` — JSON-based index at `~/.local/share/hemli/index.json` tracking all cached secrets.
- `src/model.rs` — Data structures: `StoredSecret` (value, TTL, timestamps, source, `recalculate_expires_at`), `SourceType`.
- `src/error.rs` — Error types via `thiserror`.

## Code Style

- Rust edition 2024 with `rustfmt.toml`: imports use `Item` granularity, `StdExternalCrate` grouping, Unix line endings.
- Clippy warnings are treated as errors (`-D warnings`).

## Logging

Uses `tracing`/`tracing-subscriber` to stderr, controlled via `RUST_LOG` env var (e.g., `RUST_LOG=debug hemli get ...`).
