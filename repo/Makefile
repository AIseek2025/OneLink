# OneLink repo — dev and CI entrypoints
.PHONY: check build fmt clippy test ci test-contract test-persistence-smoke test-integration

check:
	cargo check --workspace

build:
	cargo build --workspace

fmt:
	cargo fmt --all --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	ONELINK_ENV=dev cargo test --workspace

test-contract:
	ONELINK_ENV=dev cargo test --workspace -p onelink-contract-tests

test-persistence-smoke:
	RUN_PERSISTENCE_SMOKE=1 ONELINK_ENV=dev cargo test --workspace -p onelink-persistence-smoke

test-integration:
	RUN_SLICE_HTTP_SMOKE=1 ONELINK_ENV=dev cargo test --workspace -p onelink-integration-tests

ci: fmt clippy test
	@echo "CI baseline passed"
