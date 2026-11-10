#!/usr/bin/env bash
# Starts the backend API and the frontend together. Ctrl-C stops both.
#
#   ./start.sh          development (frontend hot reload)
#   ./start.sh --prod   production mode, no rebuild (used by the Docker image)
set -euo pipefail
cd "$(dirname "$0")"

MODE="${1:-dev}"
export API_PORT="${API_PORT:-8787}"
export WEB_PORT="${WEB_PORT:-3000}"

# The level is built with Solana 4.1.2. Prefer that release if it is installed
# next to an older active one.
SOLANA_BIN="${SOLANA_BIN:-$HOME/.local/share/solana/install/releases/4.1.2/solana-release/bin}"
if [ -d "$SOLANA_BIN" ]; then
  export PATH="$SOLANA_BIN:$PATH"
fi
if ! command -v cargo-build-sbf >/dev/null; then
  echo "cargo-build-sbf not found. Install the Solana CLI 4.1.2 first." >&2
  exit 1
fi

if [ "$MODE" != "--prod" ]; then
  (cd backend && cargo build --release -p api -p harness)
  (cd frontend && bun install --silent)
fi

pids=()
cleanup() {
  trap - EXIT INT TERM
  kill "${pids[@]}" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

(
  cd backend
  PORT="$API_PORT" exec "${API_BIN:-target/release/api}"
) &
pids+=($!)

(
  cd frontend
  export PORT="$WEB_PORT" BACKEND_URL="http://127.0.0.1:$API_PORT"
  if [ "$MODE" = "--prod" ]; then
    NODE_ENV=production exec bun index.ts
  else
    exec bun --hot index.ts
  fi
) &
pids+=($!)

echo "open http://localhost:$WEB_PORT"
# Exit as soon as either process stops, which takes the other one down too.
wait -n
