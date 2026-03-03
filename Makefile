.PHONY: build-sbf test

build-sbf:
	cargo-build-sbf --manifest-path stake_deposit_interceptor/Cargo.toml

test:
	make build-sbf && \
	cp ./target/sbpf-solana-solana/release/stake_deposit_interceptor_program.so ./stake_deposit_interceptor/tests/fixtures/ && \
	SBF_OUT_DIR=$(pwd)/target/sbpf-solana-solana/release cargo nextest run
