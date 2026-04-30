.PHONY: fmt fmt-check test clippy package-smoke install-smoke check release-dry-run release

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

install-smoke:
	rm -rf target/install-smoke
	cargo install --path . --root target/install-smoke --locked --debug
	target/install-smoke/bin/pg-logstats --version
	target/install-smoke/bin/pg-logstats --help >/dev/null

check: fmt-check test clippy package-smoke

release-dry-run:
	cargo release $${LEVEL:-patch} --no-push --no-publish

release:
	@BRANCH=$$(git rev-parse --abbrev-ref HEAD); \
	if [ "$$BRANCH" != "main" ]; then \
		echo "Error: must be on main branch (currently on $$BRANCH)"; \
		exit 1; \
	fi
	@echo "Creating release with version bump: $${LEVEL:-patch}"
	@echo "The pushed tag will trigger .github/workflows/release.yml"
	cargo release $${LEVEL:-patch} --execute --no-publish
