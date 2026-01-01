ifneq (,$(wildcard .env))
include .env
export
endif

.PHONY: help dev test format

help:
	@echo "Available targets:"
	@echo "  dev: Start the development server"
	@echo "  test: Run the test suite"
	@echo "  format: Format the code using cargo fmt"

dev:
	cargo run

test:
	cargo test

format:
	cargo fmt
	cargo check
	cargo clippy

