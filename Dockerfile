FROM rust:1.67 as builder
WORKDIR /usr/src/mina-indexer
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/mina-indexer/target/release/mina-indexer /usr/local/bin/mina-indexer
CMD ["mina-indexer"]
