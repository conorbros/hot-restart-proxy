FROM rust:latest as builder

WORKDIR ./hot-restart-proxy

COPY ./ ./

RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update -y \
    && apt-get install glibc-doc -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /hot-restart-proxy/target/release/hot-restart-proxy /hot-restart-proxy

ENTRYPOINT ["./hot-restart-proxy"]
