# Build the static relay binary in a full Rust toolchain image.
FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY src src

RUN cargo build --release

# Ship the compiled binary in a minimal runtime image with CA roots for outbound HTTPS.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/relay /usr/local/bin/relay

EXPOSE 8080

CMD ["relay"]