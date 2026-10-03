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
	@echo "  secret                  Generate a new 32-byte hex secret (copied to clipboard)"
	@echo "  encrypt <plaintext>     Encrypt the provided plaintext"
	@echo "  decrypt <ciphertext>    Decrypt the provided ciphertext"
	@echo "  hash <input>            Hash the provided input"

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

secret:
	@openssl rand -hex 32 | tr -d "\n" | pbcopy
	@echo "Generated a new 32-byte hex secret and copied it to clipboard."

encrypt:
	@echo "$(word 2,$(MAKECMDGOALS))" | cargo run --quiet --bin secret -- encrypt --key-env SECRET_KEY

decrypt:
	@echo "$(word 2,$(MAKECMDGOALS))" | cargo run --quiet --bin secret -- decrypt --key-env SECRET_KEY

hash:
	@echo "$(word 2,$(MAKECMDGOALS))" | cargo run --quiet --bin secret -- hash --key-env SECRET_KEY

# Swallow the extra positional argument of encrypt/decrypt/hash.
%:
	@:
