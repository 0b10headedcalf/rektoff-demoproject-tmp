# syntax=docker/dockerfile:1
# One container holding the API, the grader, the Solana toolchain and the
# frontend. Build from the repo root:
#
#   docker build -t rektoff-practice .
#   docker run --rm -p 3000:3000 --memory 4g --cpus 4 --pids-limit 512 rektoff-practice
#
# Each level's checks are compiled into the grader in the first stage. The
# level folders are copied into the final image as they are.

ARG RUST_VERSION=1.99
ARG BUN_VERSION=1.3
ARG SOLANA_VERSION=4.1.2

FROM oven/bun:${BUN_VERSION} AS bun

# ---- Stage 1: compile the API and the grader -------------------------------
FROM rust:${RUST_VERSION}-bookworm AS builder
WORKDIR /src/backend
COPY backend/ ./
RUN cargo build --release --locked -p api -p grader

# ---- Stage 2: runtime ------------------------------------------------------
FROM rust:${RUST_VERSION}-bookworm
ARG SOLANA_VERSION

RUN apt-get update \
 && apt-get install -y --no-install-recommends bzip2 ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --create-home app \
 && chown -R app:app /usr/local/rustup /usr/local/cargo
COPY --from=bun /usr/local/bin/bun /usr/local/bin/bun

USER app
ENV HOME=/home/app
ENV PATH=/home/app/.local/share/solana/install/active_release/bin:$PATH
RUN curl -sSfL "https://release.anza.xyz/v${SOLANA_VERSION}/install" | sh \
 && cargo-build-sbf --version

WORKDIR /app
COPY --chown=app:app backend/levels backend/levels

# Compile every level once. This downloads platform-tools and all crates, so
# player submissions later only recompile one file and need no network.
RUN set -eu; \
    for program in backend/levels/*/program; do \
      (cd "$program" && cargo-build-sbf); \
    done

COPY --from=builder /src/backend/target/release/api /src/backend/target/release/grader /app/bin/

COPY --chown=app:app frontend/package.json frontend/bun.lock frontend/
RUN cd frontend && bun install --frozen-lockfile --production
COPY --chown=app:app frontend/ frontend/
COPY --chown=app:app start.sh ./

ENV API_BIN=/app/bin/api \
    GRADER_BIN=/app/bin/grader \
    CARGO_NET_OFFLINE=true \
    WEB_PORT=3000
EXPOSE 3000
CMD ["./start.sh", "--prod"]
