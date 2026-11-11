# Rektoff Practice (demo)

Fix a planted bug in a Solana program in the browser, submit, and tests attack
the compiled result.

- `frontend/`: Bun server and CodeMirror page. Forwards `/api/*` to the backend.
- `backend/api/`: Rust (axum) API. Queues submissions, compiles them, runs the grader.
- `backend/grader/`: loads a compiled program into LiteSVM and runs a level's checks.
- `backend/levels/`: one folder per level.

## Add a level

Add a folder to `backend/levels/` and rebuild. The grader picks it up by itself.

```
backend/levels/level-04-some-bug/
  description.md       shown next to the editor; starts with "# Level 4: ..."
  program/             the buggy Anchor crate; players edit src/lib.rs
  solution/lib.rs      the reference fix
  tests/checks.rs      the attack, plus checks that normal use still works
```

Copy an existing level to start. Give the program a new id in `declare_id!` and
use the same id in `checks.rs`. The folder name is the level id, and levels
appear in name order. Helpers for checks are in `backend/grader/src/support.rs`.

## Run locally

Needs Rust, Bun, and the Solana CLI 4.1.2 (`cargo-build-sbf`).

```
./start.sh
```

Then open http://localhost:3000. Ports: `WEB_PORT` (3000), `API_PORT` (8787).

## Run in Docker

```
docker build -t rektoff-practice .
docker run --rm -p 3000:3000 --memory 4g --cpus 4 --pids-limit 512 rektoff-practice
```

The single container is for demos. Before strangers use it, compilation must
move into its own sandbox (gVisor or Firecracker) with no network.
