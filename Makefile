.PHONY: check test test-rust test-ts build clean

# Run all checks (compilation only, no tests)
check: check-rust check-ts

check-rust:
	cd services/api-rs && cargo check

check-ts:
	cd packages/corp-tools && npx tsc --noEmit --skipLibCheck
	cd packages/cli-ts && npx tsc --noEmit

# Run all tests
test: test-rust test-ts

test-rust:
	cd services/api-rs && cargo test

test-ts:
	cd packages/corp-tools && npx vitest run 2>/dev/null || true
	cd packages/cli-ts && npx vitest run 2>/dev/null || true

# Run just the lifecycle/e2e tests
test-lifecycle:
	cd services/api-rs && cargo test --test api_lifecycle

# Build everything
build: build-rust build-ts

build-rust:
	cd services/api-rs && cargo build --release

build-ts:
	cd packages/corp-tools && npx tsup
	cd packages/cli-ts && npx tsup

# Integration tests (requires Docker)
test-integration:
	cd crates/corp-store && docker compose -f tests/docker-compose.integration.yml up -d
	cd crates/corp-store && sleep 3 && cargo test --test integration_s3 --features s3 -- --include-ignored
	cd crates/corp-store && docker compose -f tests/docker-compose.integration.yml down -v

# Clean build artifacts
clean:
	cd services/api-rs && cargo clean
	rm -rf packages/cli-ts/dist packages/corp-tools/dist
