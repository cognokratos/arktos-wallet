.PHONY: help dev format

help:
	@echo "Available targets:"
	@echo "  dev: Start the development server"
	@echo "  format: Format the code using cargo fmt"

dev:
	RUST_LOG=debug cargo run

format:
	cargo fmt
