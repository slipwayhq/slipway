FROM rust:1.86 AS builder
WORKDIR /usr
COPY ./src ./src
WORKDIR /usr/src
RUN cargo build --release -p slipway

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates libsixel-bin fontconfig tzdata jq && \
    update-ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/target/release/slipway /usr/local/bin/slipway
