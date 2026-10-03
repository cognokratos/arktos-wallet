ifneq (,$(wildcard .env))
include .env
export
endif

IMAGE ?= arktos-wallet:dev

.PHONY: help dev build test nextest check lint fmt fmt-check fix audit deny ci \
	docker-build docker-lint docker-run sql secret encrypt decrypt hash

help:
	@echo "Development:"
	@echo "  dev           Run the server locally with debug logging"
	@echo "  build         Build the debug binaries"
	@echo "  test          Run the test suite with cargo test"
	@echo "  nextest       Run the test suite with cargo-nextest (+ doctests), as CI does"
	@echo ""
	@echo "Code quality:"
	@echo "  fmt           Format the code"
	@echo "  fmt-check     Check formatting without modifying files"
	@echo "  check         cargo check all targets and features"
	@echo "  lint          Clippy on all targets and features, warnings denied"
	@echo "  fix           Apply automatic clippy/compiler fixes"
	@echo "  audit         Check dependencies for known vulnerabilities (cargo-audit)"
	@echo "  deny          Check advisories, licenses, bans and sources (cargo-deny)"
	@echo "  ci            Run all local CI checks (no Docker required)"
	@echo ""
	@echo "Docker:"
	@echo "  docker-build  Build the Docker image ($(IMAGE))"
	@echo "  docker-lint   Lint the Dockerfile with hadolint"
	@echo "  docker-run    Run the Docker image"
	@echo ""
	@echo "Secrets & database:"
	@echo "  sql                     Open the encrypted database using sqlcipher"
	@echo "  secret                  Print a new random 32-byte base64 key (MASTER_KEY / DATABASE_KEY)"
	@echo "  encrypt <plaintext>     Encrypt a recovery phrase with MASTER_KEY"
	@echo "  decrypt <ciphertext>    Decrypt a stored recovery phrase"
	@echo "  hash <api-key>          Print the stored HMAC of an API key"

dev:
	@RUST_LOG=debug cargo run --bin arktos-wallet

build:
	cargo build --all-targets --all-features

test:
	cargo test --all-features

nextest:
	cargo nextest run --all-features
	cargo test --doc --all-features

check:
	cargo check --all-targets --all-features

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

fix:
	cargo clippy --fix --allow-dirty --all-targets --all-features
	cargo fix --allow-dirty --all-targets --all-features

audit:
	cargo audit

deny:
	cargo deny check

# Mirrors the Rust jobs in .github/workflows/ci.yml; compiler warnings are errors.
ci: export CARGO_BUILD_WARNINGS = deny
ci: fmt-check check lint nextest audit deny

docker-build:
	docker build -t $(IMAGE) .

docker-lint:
	hadolint Dockerfile

docker-run:
	@docker run -it --rm --name arktos-wallet -p 8080:8080 -v ./infra/data:/data --env-file infra/.env $(IMAGE)

sql:
	@sqlcipher "$(DATABASE_PATH)" -cmd "PRAGMA key = '$$DATABASE_KEY';"

# Portable (no openssl/pbcopy needed): 32 bytes from the OS RNG, base64-encoded.
secret:
	@cargo run --quiet --bin secret -- generate-key

encrypt:
	@printf '%s' '$(word 2,$(MAKECMDGOALS))' | cargo run --quiet --bin secret -- encrypt

decrypt:
	@printf '%s' '$(word 2,$(MAKECMDGOALS))' | cargo run --quiet --bin secret -- decrypt

hash:
	@printf '%s' '$(word 2,$(MAKECMDGOALS))' | cargo run --quiet --bin secret -- hash

# Swallow the extra positional argument of encrypt/decrypt/hash.
%:
	@:
