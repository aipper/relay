# Repository Guidelines

## Overview

This repo is a multi-component system to run AI coding CLIs on host machines and manage them remotely from a PWA.

- **`server/` (Rust)**: central controller (auth, routing, storage, WebSocket for PWA and hosts)
- **`hostd/` (Rust)**: host daemon (PTY process runner, event spool, connects outbound to `server`)
- **`cli/` (Bun)**: local command wrapper (starts runs via `hostd`, does not directly own tool processes)
- **`web/` (Svelte PWA)**: mobile-friendly UI (runs list, live logs, approve/deny/input)
- **`docs/`**: protocols, API notes, operational docs

## Build, Test, and Dev Commands

Rust (workspace):

- `cargo fmt` / `cargo fmt --check`: format Rust code
- `cargo clippy --all-targets --all-features`: lint Rust code
- `cargo test`: run Rust tests

Bun CLI:

- `cd cli && bun install`: install dependencies
- `cd cli && bun run dev`: run CLI in dev mode (if provided)

Web (Svelte PWA):

- `cd web && bun install`: install dependencies
- `cd web && bun run dev`: start dev server
- `cd web && bun run build`: build production assets

## Coding Style & Naming

- Keep changes scoped; avoid refactors unless required.
- Rust: `rustfmt` + clippy-clean; prefer explicit types at API boundaries.
- TypeScript: `eslint`/`prettier` if configured; keep file/module names kebab-case, exports named.
- Event types are stable API: add new fields compatibly; never remove/rename without a version bump.

## Testing Guidelines

- Prefer fast unit tests for pure logic (redaction, protocol parsing, routing).
- Add integration tests for host↔server event flow when stable.
- Test names should describe behavior (e.g., `redacts_bearer_tokens`).

## Commit & PR Guidelines

- Use clear, scoped messages (e.g., `server: add ws auth`, `hostd: spool ack logic`).
- PRs should include: what changed, how to verify, and any rollback steps.

## Security & Configuration

- Never commit secrets. Keep runtime config in `conf/env` (or `.env`) and add to `.gitignore`.
- Log retention is short by default (3 days). Inputs are stored **redacted**; raw input storage is off by default.
