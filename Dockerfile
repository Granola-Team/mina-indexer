FROM rust:1.75 as builder
WORKDIR /usr/src/mina-indexer
COPY . .
RUN apt-get update && apt-get install -y libclang-dev && rm -rf /var/lib/apt/lists/*
RUN RUST_BACKTRACE=1 cargo build --release

FROM debian:bullseye-slim as runner
RUN apt-get update && apt-get install -y openssl ca-certificates curl python3 && rm -rf /var/lib/apt/lists/*

RUN curl -O https://dl.google.com/dl/cloudsdk/channels/rapid/downloads/google-cloud-sdk-410.0.0-linux-x86_64.tar.gz
RUN tar xzf google-cloud-sdk-410.0.0-linux-x86_64.tar.gz && rm google-cloud-sdk-410.0.0-linux-x86_64.tar.gz
RUN ln -s /lib /lib64
ENV PATH="/google-cloud-sdk/bin:${PATH}"

COPY --from=builder /usr/src/mina-indexer/target/release/mina-indexer /usr/local/bin/mina-indexer
ENTRYPOINT ["mina-indexer"]
