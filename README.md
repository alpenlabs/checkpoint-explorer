# Checkpoint Explorer

## Running the full stack with Docker

```sh
docker compose up --build -d
```

Starts three containers — MariaDB, the Rust backend (port `3000`), and the frontend (port `5173`). Migrations run automatically at backend startup.

To tear down and wipe the database:

```sh
docker compose down -v
```

## Running the backend using binary

Start only the database:

```sh
docker compose up -d mariadb
```

Then run the backend:

```sh
cd backend
cargo run --bin checkpoint-explorer
```

The backend reads config from environment variables — see `backend/.env.example`.

## Running the frontend using npm

```sh
cd frontend
npm install
npm run dev -- --host
```

## Prerequisites

- Rust (stable) — [rustup](https://rustup.rs)
- Node.js ≥ 18 — [nvm](https://github.com/nvm-sh/nvm)
- [just](https://github.com/casey/just) — `cargo install just`

## Functional tests

Requires [uv](https://github.com/astral-sh/uv) and Docker (the test suite spins up an ephemeral MariaDB container).

```sh
cd functional-tests
uv run python entry.py
```

## Code quality

```sh
just check      # fmt + clippy + ruff — run before pushing
just lint-fix   # auto-fix clippy suggestions
just fmt        # auto-format Rust code
```
