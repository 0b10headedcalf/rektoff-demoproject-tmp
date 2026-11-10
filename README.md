# Rektoff Practice (demo)

Fix a planted bug in a Solana program in the browser, submit, and hidden tests
attack the compiled result.

- `frontend/`: Bun server and CodeMirror page. Forwards `/api/*` to the backend.
- `backend/api/`: Rust (axum) API. Queues submissions, compiles them, runs the grader.
- `backend/grader/`: loads a compiled program into LiteSVM and runs a level's checks.
- `backend/levels/`: one folder per level.

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
