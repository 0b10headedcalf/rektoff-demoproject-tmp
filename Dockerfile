# syntax=docker/dockerfile:1
# One container holding the API, the grader, the Solana toolchain and the
# frontend. Build from the repo root:
#
#   docker build -t rektoff-practice .
#   docker run --rm -p 3000:3000 --memory 4g --cpus 4 --pids-limit 512 rektoff-practice
#
# Hidden tests and reference fixes are compiled into the grader in the first
# stage and are NOT copied into the final image, so a submission cannot read
# them from disk.

ARG RUST_VERSION=1.99
ARG BUN_VERSION=1.3
ARG SOLANA_VERSION=4.1.2

FROM oven/bun:${BUN_VERSION} AS bun

# ---- Stage 1: compile the API and the grader -------------------------------
FROM rust:${RUST_VERSION}-bookworm AS builder
WORKDIR /src/backend
COPY backend/ ./
RUN cargo build --release --locked -p api -p harness

# Keep only what players are allowed to see from each level.
RUN set -eu; \
    for level in levels/*/; do \
      name="$(basename "$level")"; \
      mkdir -p "/public-levels/$name"; \
      cp -r "$level/program" "/public-levels/$name/program"; \
      cp "$level/description.md" "/public-levels/$name/"; \
    done

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
COPY --from=builder --chown=app:app /public-levels backend/levels

# Compile every level once. This downloads platform-tools and all crates, so
# player submissions later only recompile one file and need no network.
RUN set -eu; \
    for program in backend/levels/*/program; do \
      (cd "$program" && cargo-build-sbf); \
    done

COPY --from=builder /src/backend/target/release/api /src/backend/target/release/harness /app/bin/

COPY --chown=app:app frontend/package.json frontend/bun.lock frontend/
RUN cd frontend && bun install --frozen-lockfile --production
COPY --chown=app:app frontend/ frontend/
COPY --chown=app:app start.sh ./

ENV API_BIN=/app/bin/api \
    HARNESS_BIN=/app/bin/harness \
    CARGO_NET_OFFLINE=true \
    WEB_PORT=3000
EXPOSE 3000
CMD ["./start.sh", "--prod"]
