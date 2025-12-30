# Personal Operating System - Coding Guidelines

## Project Overview

A Kanban-based task management webapp built with Rust, featuring boards, columns, cards, tags, and a full REST API. All cryptographic operations (password hashing) happen server-side using Argon2.

## Build Commands

**Primary commands** (via Makefile):
- `make install` - Install all dependencies (sqlx-cli, taplo)
- `make build` - Build release binary
- `make dev` - Run development server
- `make test` - Run all tests
- `make lint` - Lint all code (clippy)
- `make format` - Format all code (Rust + TOML)
- `make migrate` - Run database migrations

**Individual tasks**:
- Rust: `cargo +nightly test|clippy|fmt`
- TOML: `taplo format`

## Architecture

The system follows a layered architecture:
- **handlers/** - HTTP request handlers (axum routes)
- **models/** - Domain models and DTOs
- **repo/** - Repository layer (database access via sqlx)
- **auth/** - Authentication module (password hashing, session management)
- **templates/** - Server-rendered HTML (Askama)

## Code Structure

```
src/
|-- main.rs          # Application entry point and router
|-- error.rs         # Error types and handling
|-- state.rs         # Application state (repositories)
|-- auth/            # Authentication (extractors, password utils)
|-- handlers/        # HTTP handlers (auth, boards, columns, cards, tags, web)
|-- models/          # Domain models (user, board, column, card, tag, etc.)
|-- repo/            # Repository layer (user, board, column, card, tag, etc.)
migrations/          # SQLite migrations (sqlx)
templates/           # HTML templates (Askama/Bootstrap)
tests/               # Integration tests
```

## Development Standards

**Conventions**:
- Makefile targets have `.PHONY` declarations
- Use only Makefile targets for build/test/lint/format
- Avoid UTF-8 emoji/special characters in code and docs
- Nightly Rust toolchain (specified in `rust-toolchain.toml`)
- Use Bootstrap Icons CSS classes for icons (no emoji)

**Formatting requirements**:
- Run `make format` before committing
- Rust: `cargo +nightly fmt`
- TOML: `taplo format`

**Pre-commit checklist**:
1. Execute `make test`
2. Execute `make format`
3. Execute `make lint`

**Commit standards**:
- No emojis in messages
- Wrap titles at 72 characters
- Wrap body at 80 characters
- Use conventional prefixes: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`

**Code style**:
- Follow clippy lints with `-D warnings`
- Keep functions focused and small
- Prefer explicit error handling over panics in library code
- Use Bootstrap Icons exclusively for all icons
- Use UUIDs for all entity IDs
- Use chrono for dates/timestamps

## Database

- SQLite with sqlx for type-safe queries
- Migrations in `migrations/` folder
- Repository pattern for database access (easy to swap DB later)

## Authentication

- Email + password with Argon2 hashing
- Session cookies for web UI
- Bearer API tokens for programmatic access
- Token scopes: read, write, admin

## Authorization

- Board roles: owner, editor, reader
- Card visibility: private (editors/owners only), restricted (board members), public (feature-flagged)

## API

- REST API at `/api/*`
- JSON request/response bodies
- Session cookie or Bearer token authentication
- Full documentation in `docs/api.md`
