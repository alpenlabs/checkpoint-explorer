# AGENTS.md

This file provides guidance to AI coding assistants when working with code in this repository.

## Overview

**Checkpoint Explorer** is a full-stack explorer for Strata checkpoints. It has:

- a Rust backend that fetches Strata checkpoint data and block headers from a Strata fullnode, stores them in MariaDB, and exposes HTTP API endpoints;
- a React/Vite frontend that renders checkpoint search, lists, pagination, and detail views;
- Python functional tests that spin up an isolated environment and exercise the backend API.

The full stack is normally run through Docker Compose. The backend listens on port `3000`, the frontend on port `5173`, and MariaDB on port `3306`.

The bottom line for future changes: preserve the user-visible behavior and API contract, but do not preserve weak implementation patterns just because they are present today. When touching an area, leave it closer to clear, typed, testable, production-quality code.

## Repository Layout

| Path | Purpose |
|------|---------|
| `backend/` | Rust workspace for the API server, database layer, migrations, model entities, and fullnode client |
| `backend/bin/checkpoint-explorer/` | Main Axum server binary and background fetcher orchestration |
| `backend/database/` | SeaORM database connection and query/service helpers |
| `backend/fullnode-client/` | JSON-RPC client/fetcher for the Strata fullnode |
| `backend/migration/` | SeaORM migrations for MariaDB schema setup |
| `backend/model/` | SeaORM entity models and RPC/API data types for checkpoints and block headers |
| `frontend/` | React 18 + Vite + TypeScript frontend |
| `frontend/public/config.json` | Runtime frontend config loaded by `ConfigProvider` |
| `functional-tests/` | Python `flexitest` test harness and API-level tests |
| `.justfile` | Common local development commands |
| `docker-compose.yml` | Full-stack MariaDB, backend, and frontend composition |

## Backend Architecture

The backend workspace members are defined in `backend/Cargo.toml`:

| Crate | Description |
|-------|-------------|
| `bin/checkpoint-explorer` | Application binary. Loads config, runs migrations, starts background fetchers, and serves API routes. |
| `database` | Database wrapper and services for checkpoint/block-header pagination and lookup. |
| `fullnode-client` | Strata fullnode RPC fetcher. |
| `migration` | SeaORM migration definitions. |
| `model` | SeaORM generated-style entity definitions. |

The server exposes API routes under `/api`:

- `GET /api/checkpoints`
- `GET /api/checkpoint`
- `GET /api/search`

Startup also spawns background tasks for Strata checkpoint fetching, block-header fetching, and checkpoint status updates. Configuration comes from CLI args and environment variables via `clap`; useful variables include:

- `STRATA_FULLNODE`
- `APP_DATABASE_URL`
- `APP_FETCH_INTERVAL`
- `APP_STATUS_UPDATE_INTERVAL`
- `APP_SERVER_PORT`
- `RUST_LOG`

Current backend domain details:

- Checkpoints are **Strata checkpoints** from `strata_getCheckpointInfo`.
- Chain status comes from `strata_getChainStatus`.
- L2 block-header data comes from `strata_getHeadersInRange`; the project does not fetch or store full blocks.
- The database table/crate names still use `block`/`blocks` in places, but the stored payload is a header index: hash, height/slot, and checkpoint index.
- Search maps an L2 block height or L2 block hash to the containing checkpoint index using the stored header index.

## Frontend Architecture

The frontend is a Vite app using React, TypeScript, React Router, React Query, React Bootstrap, and MUI base/system packages.

Important paths:

- `src/App.tsx` wires the main application view.
- `src/providers/ConfigProvider.tsx` loads `/config.json` at runtime.
- `src/components/` contains reusable UI components.
- `src/hooks/` contains table/config hooks.
- `src/styles/` contains CSS modules and global CSS.
- `src/types/` contains shared TypeScript types.

When changing API payloads, update both the backend response shape and the frontend types/components that consume it.

Current frontend details:

- `ConfigProvider` fetches `/config.json` at runtime and provides API/explorer URLs plus refresh interval.
- The checkpoint table polls `/api/checkpoints` and the detail view reads `/api/checkpoint?p=<idx>`.
- Search calls `/api/search?query=<value>` and navigates to a checkpoint detail page.
- Some frontend code is transitional: stale fields such as `l2_blockid`, `any` type guards, `console.log`, duplicated fetch logic, and navigation/query-param coupling should be cleaned up when nearby code changes.

## Development Commands

Run commands from the repository root unless noted otherwise.

### Full Stack

```bash
docker compose up --build -d
docker compose down -v
```

### Backend

```bash
docker compose up -d mariadb
cd backend && cargo run --bin checkpoint-explorer
cd backend && cargo build
cd backend && cargo fmt
cd backend && cargo fmt --check
cd backend && cargo clippy --workspace --lib --bins --tests --no-deps -- -D warnings -W clippy::uninlined_format_args
```

The root `.justfile` wraps common backend checks:

```bash
just fmt
just fmt-check
just lint
just lint-fix
just build
```

### Frontend

```bash
cd frontend && npm install
cd frontend && npm run dev -- --host
cd frontend && npm run build
cd frontend && npm run lint
cd frontend && npm run format:check
```

### Functional Tests

```bash
cd functional-tests && uv run python entry.py
cd functional-tests && uv run python entry.py --list-tests
cd functional-tests && uv run python entry.py -t checkpoints
```

The root shortcuts are:

```bash
just test
just lint-py
just check
```

`just check` currently runs Rust formatting checks, Rust clippy, and Python Ruff checks. It does not run the frontend lint/build commands.

## Engineering Guidelines

### Quality Bar

- Improve code as you touch it. The repo has working but imperfect patterns; do not copy weak patterns into new code.
- Preserve behavior first. Use functional tests, focused unit tests, or frontend build/lint checks to prove the same external behavior still works.
- Prefer explicit types and narrow interfaces over `any`, stringly typed sentinels, and implicit response shapes.
- Prefer returning structured errors over swallowing failures, returning default empty values, or panicking in service code.
- Keep refactors incremental. If a broad cleanup is needed, separate it from behavior changes or document why it must be done together.
- Delete dead code and misleading comments in touched areas. Update names when they are actively misleading, especially around `block` versus block-header concepts.

### General

- Keep changes scoped to the layer being modified. Avoid unrelated refactors while fixing API, UI, migration, or test issues.
- Respect the existing workspace split: entity definitions in `model`, persistence/query logic in `database`, Strata RPC access in `fullnode-client`, and HTTP/task orchestration in the binary crate.
- Follow existing module boundaries, but upgrade implementation quality within them. Existing local style is context, not a ceiling.
- Prefer simple, idiomatic code over adding new abstractions. Add an abstraction only when it removes real duplication or makes invariants easier to enforce.
- Do not modify generated lockfiles or dependency manifests unless the task requires dependency changes.
- The worktree may contain user changes. Inspect before editing and do not revert unrelated modifications.

### Rust

- Use `anyhow` in application-level code where contextual propagation is useful.
- Prefer domain-specific `Result` returns in library/service code instead of logging and returning `None`, `false`, or empty collections on database/RPC errors.
- Avoid `panic!`, `unwrap()`, and `expect()` in long-running service paths. Use them only for impossible invariants or startup failures where crashing is intentional.
- Use structured `tracing` fields for logs, for example `info!(%addr, "Server started")`.
- Keep route handlers thin: parse query/state, call a service, and convert the service result into a response.
- Keep database access in `database::services` where practical; do not spread SeaORM query details into HTTP handlers.
- Keep RPC details inside `fullnode-client`; callers should work with typed domain structs.
- Preserve checkpoint continuity and header continuity invariants, but handle violations as recoverable service errors where possible.
- Treat `blocks` naming carefully. New code should say `header`, `block_header`, or `l2_header` when referring to data fetched by `strata_getHeadersInRange`.
- Run `cargo fmt` after Rust edits and clippy when touching backend behavior.
- When adding or changing schema, create/update SeaORM migrations and keep `model` entities in sync.
- Add focused Rust unit tests for parsing/conversion logic and service helpers when behavior is nontrivial.

### TypeScript/React

- Keep runtime configuration in `public/config.json` and the `ConfigProvider` contract.
- Keep API response types explicit in `src/types` or near the consuming hook/component when local. Remove stale fields when the backend no longer returns them.
- Avoid `any` in new code. Use `unknown` plus type narrowing for untrusted JSON, or typed API helpers when possible.
- Centralize repeated fetch/response parsing logic in hooks or small API helpers rather than duplicating it across components.
- Remove debug `console.log` statements from production UI code. Keep user-facing error/loading states explicit.
- Preserve the current CSS-module approach for component styling unless the task is a deliberate UI architecture change.
- Keep pagination/query-param behavior stable when refactoring. The list route uses 1-based pages; the detail route currently uses checkpoint indexes as `p`.
- Run `npm run build` for TypeScript-sensitive changes and `npm run lint` for frontend code changes.

### Python Functional Tests

- Functional tests use `flexitest` and environment services under `functional-tests/envs`.
- The test environment starts a mock Strata fullnode, a MariaDB Docker container, and the compiled backend binary.
- The mock fullnode serves deterministic `strata_getChainStatus`, `strata_getCheckpointInfo`, and `strata_getHeadersInRange` responses.
- Put API client helpers in `functional-tests/utils` instead of duplicating request logic across tests.
- Run targeted tests with `uv run python entry.py -t <test_name>` when possible, then broader tests for shared environment changes.
- Add or update functional tests for API behavior changes, sync behavior, search behavior, pagination, status transitions, and checkpoint/header mapping.
- Ruff is configured in `functional-tests/pyproject.toml`; use `just lint-py` for style checks.

## Verification Guidance

Choose checks based on what changed:

| Change area | Minimum useful checks |
|-------------|-----------------------|
| Rust formatting only | `just fmt-check` |
| Backend logic/API/schema | `cd backend && cargo test`, `just lint`, and targeted functional tests |
| Full sync behavior | `cd backend && cargo build`, then `cd functional-tests && uv run python entry.py` |
| Frontend TypeScript/components | `cd frontend && npm run build` and `cd frontend && npm run lint` |
| Python tests/harness | `just lint-py` and the affected `uv run python entry.py -t <test_name>` |

CI currently runs Rust build, Rust fmt/clippy, Python Ruff, and functional tests. Frontend build/lint is not covered by the root `just check`, so run frontend checks manually when touching `frontend/`.

## Git and Review Notes

- Prefer small, focused commits. Conventional Commits are a good default when commit style is not otherwise specified.
- Before handing off code changes, report which checks were run and any checks that were skipped.
- If Docker, `npm install`, `uv`, or other network-backed commands fail because of environment restrictions, state that clearly and continue with checks that do not require network access.
