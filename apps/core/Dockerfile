FROM rust:1.96-bookworm AS builder
WORKDIR /workspace
COPY Cargo.toml Cargo.lock ./
COPY apps/core ./apps/core
RUN cargo build --release --package baia-core

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --create-home baia
WORKDIR /app
COPY --from=builder /workspace/target/release/baia-core /usr/local/bin/baia-core
USER baia
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/baia-core"]
