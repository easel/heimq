# syntax=docker/dockerfile:1

# Keep the builder image aligned with rust-toolchain.toml.
FROM rust:1.85.1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build -p heimq --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /var/lib/heimq --shell /usr/sbin/nologin heimq \
    && mkdir -p /data \
    && chown -R heimq:heimq /data /var/lib/heimq

COPY --from=builder /app/target/release/heimq /usr/local/bin/heimq

USER heimq
WORKDIR /var/lib/heimq

ENV HEIMQ_HOST=0.0.0.0
ENV HEIMQ_DATA_DIR=/data

EXPOSE 9092 9093

ENTRYPOINT ["/usr/local/bin/heimq"]
