.PHONY: check fmt lint test doc audit deny cli build clean

check: fmt lint test doc audit deny cli

build:
	cargo build --workspace

fmt:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

audit:
	cargo audit

deny:
	cargo deny check

cli:
	cargo test -p soroban-testkit-cli

clean:
	cargo clean
