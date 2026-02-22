.PHONY: build-sbf clean test clippy fmt idl-build client-build

build-sbf:
	cargo-build-sbf --manifest-path stake_deposit_interceptor/Cargo.toml
