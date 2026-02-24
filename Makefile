.PHONY: build-sbf test

build-sbf:
	cargo-build-sbf --manifest-path stake_deposit_interceptor/Cargo.toml

test:
	cargo nextest run
