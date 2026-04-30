.PHONY: fmt fmt-check test clippy package-smoke check

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

test:
	cargo test

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

package-smoke:
	cargo package --allow-dirty --no-verify

check: fmt-check test clippy package-smoke
