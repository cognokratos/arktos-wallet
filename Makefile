ifneq (,$(wildcard .env))
include .env
export
endif

.PHONY: help dev test format fix sql secret encrypt decrypt

help:
	@echo "Available targets:"
	@echo "  dev: Start the development server"
	@echo "  test: Run the test suite"
	@echo "  format: Format the code and check for linting issues"
	@echo "  fix: Automatically fix linting issues"
	@echo "  sql: Open the encrypted database using sqlcipher"
	@echo "  secret: Generate a new 32-byte hex secret"
	@echo "  encrypt <plaintext>: Encrypt the provided plaintext"
	@echo "  decrypt <ciphertext>: Decrypt the provided ciphertext"

dev:
	@cargo run --bin arktos-wallet

test:
	@cargo test

format:
	@cargo fmt
	@cargo check
	@cargo clippy

fix:
	@cargo fix --allow-dirty --allow-staged

sql:
	@sqlcipher "$(DATABASE_PATH)" -cmd "PRAGMA key = '$$DATABASE_KEY';"

secret:
	@openssl rand -hex 32 | tr -d "\n" | pbcopy
	@echo "Generated a new 32-byte hex secret and copied it to clipboard."

encrypt:
	@echo "$(word 2,$(MAKECMDGOALS))" | cargo run --quiet --bin secret -- encrypt --key-env SECRET_KEY

decrypt:
	@echo "$(word 2,$(MAKECMDGOALS))" | cargo run --quiet --bin secret -- decrypt --key-env SECRET_KEY

%:
	@:
