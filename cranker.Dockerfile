# NOTE: this must be run from the project root directory
FROM rust:1.81 AS builder

WORKDIR /usr/src/interceptor

# Copy program files for dependency
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
COPY stake_deposit_interceptor stake_deposit_interceptor
# Need `api` as it's in the same workspace as `stake_deposit_interceptor`
COPY api api

# Copy project files (excluding env)
COPY ./cranker/Cargo.lock ./cranker/Cargo.lock
COPY ./cranker/Cargo.toml ./cranker/Cargo.toml
COPY ./cranker/src ./cranker/src
RUN --mount=type=cache,mode=0777,target=/home/root/app/target \
    --mount=type=cache,mode=0777,target=/usr/local/cargo/registry \
    cargo build --release --manifest-path ./cranker/Cargo.toml

FROM debian:bookworm-slim
COPY --from=builder /usr/src/interceptor/cranker/target/release/stake-deposit-interceptor-cranker /usr/local/bin/stake-deposit-interceptor-cranker
CMD ["stake-deposit-interceptor-cranker"]
