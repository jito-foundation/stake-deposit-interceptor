FROM rust:1.81 AS builder
RUN apt-get update && apt-get install -y libudev-dev libclang-dev clang pkg-config libssl-dev build-essential cmake protobuf-compiler
RUN update-ca-certificates
WORKDIR /usr/src/app
COPY . .
RUN --mount=type=cache,mode=0777,target=/home/root/app/target \
    --mount=type=cache,mode=0777,target=/usr/local/cargo/registry \
    cargo build --release --bin stake-deposit-interceptor-cranker

#########
FROM ubuntu:22.04 as runtime
RUN apt-get update && apt-get install -y ca-certificates procps
ENV APP="stake-deposit-interceptor-cranker"
COPY --from=builder /usr/src/app/target/release/$APP ./$APP
ENTRYPOINT ./$APP
