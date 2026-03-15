# syntax=docker/dockerfile:1.7

# Build the static relay binary in a full Rust toolchain image.
FROM rust:1-bookworm AS builder

WORKDIR /app

ENV CARGO_TARGET_DIR=/target

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/target \
    cargo build --release \
    && cp /target/release/relay /app/relay

# Ship the compiled binary in a minimal runtime image with CA roots for outbound HTTPS.
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/relay /app/relay

EXPOSE 8080

CMD ["./relay"]