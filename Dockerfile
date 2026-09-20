# ---- Build stage ----
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# syntect (see Cargo.toml) pulls in the `onig` crate (Oniguruma), which is a
# C library built from source by its own build script - a C toolchain is
# needed here, but not in the runtime image below.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates

RUN cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN useradd --system --create-home --shell /usr/sbin/nologin appuser

COPY --from=builder /app/target/release/pastebin ./pastebin

USER appuser

# Actual bind address/port come from the URL/PORT env vars at runtime (see
# src/main.rs) - this is metadata for tools like `docker inspect`, not a
# guarantee. Required env vars: URL, PORT, MAX_PASTES, MAX_PASTE_SIZE,
# MAX_FILE_SIZE (the app panics at startup if any is missing/invalid).
EXPOSE 3000

ENTRYPOINT ["./pastebin"]
