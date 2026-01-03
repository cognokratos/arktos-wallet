ifneq (,$(wildcard .env))
include .env
export
endif

.PHONY: help dev build run test format fix sql secret encrypt decrypt

help:
	@echo "Available targets:"
	@echo "  dev: Start the development server"
	@echo "  build: Build the Docker image"
	@echo "  run: Run the Docker container"
	@echo "  test: Run the test suite"
	@echo "  format: Format the code and check for linting issues"
	@echo "  fix: Automatically fix linting issues"
	@echo "  sql: Open the encrypted database using sqlcipher"
	@echo "  secret: Generate a new 32-byte hex secret"
	@echo "  encrypt <plaintext>: Encrypt the provided plaintext"
	@echo "  decrypt <ciphertext>: Decrypt the provided ciphertext"
	@echo "  hash <input>: Hash the provided input"

dev:
	@cargo run --bin arktos-wallet

build:
	@docker build -t arktos-wallet .

run:
	@docker run -it --rm --name arktos-wallet -p 8080:8080 -v ./infra/data:/data --env-file infra/.env arktos-wallet

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

hash:
	@echo "$(word 2,$(MAKECMDGOALS))" | cargo run --quiet --bin secret -- hash --key-env SECRET_KEY

%:
	@:
