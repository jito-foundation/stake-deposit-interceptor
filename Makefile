.PHONY: build-sbf test

build-sbf:
	cargo-build-sbf --manifest-path stake_deposit_interceptor/Cargo.toml

build-idl:
	jito-shank-cli \
        --program-env-path ./config/program.env \
        --output-idl-path ./stake_deposit_interceptor/idl/ \
        generate \
        --program-id-key "STAKE_DEPOSIT_INTERCEPTOR_PROGRAM_ID" \
        --idl-name stake_deposit_interceptor \
        --module-paths "stake_deposit_interceptor"

build-client:
	pnpm codama run --all

test:
	make build-sbf && \
	cp ./target/sbpf-solana-solana/release/stake_deposit_interceptor_program.so ./stake_deposit_interceptor/tests/fixtures/ && \
	SBF_OUT_DIR=$(pwd)/target/sbpf-solana-solana/release cargo nextest run
