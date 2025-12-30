# Personal Operating System - Makefile
# A Kanban-based task management webapp

.PHONY: all build clean dev format format-check help install lint migrate test

.DEFAULT_GOAL := help

# Configuration
DATABASE_URL ?= sqlite:data.db?mode=rwc

## help: Show this help message
help:
	@echo "Personal Operating System - Available targets:"
	@echo ""
	@grep -E '^## [a-zA-Z_-]+:.*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ": "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}' | sed 's/## /  /'

## install: Install development dependencies
install:
	cargo install sqlx-cli --no-default-features --features sqlite
	cargo install taplo-cli

## build: Build the project in release mode
build:
	cargo build --release

## dev: Run the development server with hot reload
dev:
	DATABASE_URL=$(DATABASE_URL) cargo run

## test: Run all tests
test:
	DATABASE_URL=$(DATABASE_URL) cargo test

## migrate: Run database migrations
migrate:
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run

## migrate-create: Create a new migration (usage: make migrate-create name=migration_name)
migrate-create:
	sqlx migrate add $(name)

## format: Format all code
format: format-rust format-toml

## format-rust: Format Rust code
format-rust:
	cargo +nightly fmt

## format-toml: Format TOML files
format-toml:
	taplo format

## format-check: Check code formatting
format-check: format-check-rust format-check-toml

## format-check-rust: Check Rust code formatting
format-check-rust:
	cargo +nightly fmt -- --check

## format-check-toml: Check TOML file formatting
format-check-toml:
	taplo format --check

## lint: Run linter
lint:
	cargo clippy -- -D warnings

## clean: Remove build artifacts
clean:
	cargo clean
	rm -f data.db data.db-shm data.db-wal

## db-reset: Reset the database
db-reset:
	rm -f data.db data.db-shm data.db-wal
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run

## check: Run all checks (format, lint, test)
check: format-check lint test
