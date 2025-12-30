# Personal Operating System - Makefile
# A Kanban-based task management webapp

.DEFAULT_GOAL := help

# Configuration
DATABASE_URL ?= sqlite:data.db?mode=rwc

# Dependency versions
SQLX_CLI_VERSION ?= 0.7.4
TAPLO_VERSION ?= 0.9.3

# Docker configuration
DOCKER_IMAGE ?= personal-os
DOCKER_TAG ?= latest

## help: Show this help message
.PHONY: help
help:
	@echo "Personal Operating System - Available targets:"
	@echo ""
	@grep -E '^## [a-zA-Z_-]+:.*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ": "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}' | sed 's/## /  /'

## install: Install development dependencies
.PHONY: install
install:
	cargo install sqlx-cli@$(SQLX_CLI_VERSION) --no-default-features --features sqlite
	cargo install taplo-cli@$(TAPLO_VERSION)

## build: Build the project in release mode
.PHONY: build
build:
	cargo build --release

## dev: Run the development server with hot reload
.PHONY: dev
dev:
	DATABASE_URL=$(DATABASE_URL) cargo run

## test: Run all tests
.PHONY: test
test:
	DATABASE_URL=$(DATABASE_URL) cargo test

## migrate: Run database migrations
.PHONY: migrate
migrate:
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run

## migrate-create: Create a new migration (usage: make migrate-create name=migration_name)
.PHONY: migrate-create
migrate-create:
	sqlx migrate add $(name)

## format: Format all code
.PHONY: format
format: format-rust format-toml

## format-rust: Format Rust code
.PHONY: format-rust
format-rust:
	cargo +nightly fmt

## format-toml: Format TOML files
.PHONY: format-toml
format-toml:
	taplo format

## format-check: Check code formatting
.PHONY: format-check
format-check: format-check-rust format-check-toml

## format-check-rust: Check Rust code formatting
.PHONY: format-check-rust
format-check-rust:
	cargo +nightly fmt -- --check

## format-check-toml: Check TOML file formatting
.PHONY: format-check-toml
format-check-toml:
	taplo format --check

## lint: Run linter
.PHONY: lint
lint:
	cargo clippy -- -D warnings

## clean: Remove build artifacts
.PHONY: clean
clean:
	cargo clean
	rm -f data.db data.db-shm data.db-wal

## db-reset: Reset the database
.PHONY: db-reset
db-reset:
	rm -f data.db data.db-shm data.db-wal
	DATABASE_URL=$(DATABASE_URL) sqlx migrate run

## check: Run all checks (format, lint, test)
.PHONY: check
check: format-check lint test

## docker-build: Build Docker image
.PHONY: docker-build
docker-build:
	docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .

## docker-run: Run Docker container
.PHONY: docker-run
docker-run:
	docker run -p 3000:3000 -v $(PWD)/data:/home/app/data $(DOCKER_IMAGE):$(DOCKER_TAG)

## docker-lint: Lint Dockerfile with hadolint
.PHONY: docker-lint
docker-lint:
	docker run --rm -i -v $(PWD)/.hadolint.yaml:/.hadolint.yaml hadolint/hadolint < Dockerfile
