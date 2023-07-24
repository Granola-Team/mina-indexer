FROM rust:1.70 as builder
WORKDIR /usr/src/mina-indexer
COPY . .
RUN apt-get update && apt-get install -y libclang-dev && rm -rf /var/lib/apt/lists/*
RUN RUST_BACKTRACE=1 cargo build --release

FROM debian:bullseye-slim as runner
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/mina-indexer/target/release/mina-indexer /usr/local/bin/mina-indexer
ENTRYPOINT ["mina-indexer"]
