# Personal Operating System

Personal Operating System (POS) is an open, self-hosted, LLM-driven operating system for execution and cognition externalization.

It can be used as a task manager, but it is designed to go significantly beyond traditional task management by acting as a long-term, auditable control plane for work, life, and decision-making.

POS unifies tasks, projects, habits, learning, health, travel, and investments into a single, structured system that remains observable, queryable, and automation-first over time.

## Goals

- Provide a unified system to manage tasks, projects, and long-term initiatives
- Externalize cognition in a structured, machine-readable form
- Optimize prioritization, focus, and execution over time
- Make actions, decisions, and outcomes auditable
- Enable selective transparency (public, restricted, private views)
- Serve as a durable system of record
- Act as a first-class substrate for LLM-driven agents

## LLM-Driven by Design

POS is designed to be operated both by humans and by Large Language Models.

### Core Principles

- Full, stable API covering all state transitions (create, update, move, tag, archive, report)
- Explicit domain model optimized for machine interaction
- Deterministic behavior with no hidden business logic
- The UI is a projection of the system, not the source of truth

### This Enables

- Managing tasks and Kanban boards via natural-language commands
- AI agents that create, prioritize, and reorganize work
- Automated daily and weekly reporting
- Continuous optimization of focus and throughput
- Integration with external systems (GitHub, Gitea, trackers, data sources)

LLMs are treated as operators of the system, not assistants layered on top.

## Tech Stack

- **Language**: Rust
- **Web Framework**: Axum
- **Database**: SQLite (via sqlx)
- **Templating**: Askama
- **CSS**: Bootstrap 5
- **Auth**: Argon2 password hashing

## Setup

### Prerequisites

- Rust nightly toolchain
- SQLite

### Using Docker

The easiest way to run the application:

```bash
# Build the Docker image
make docker-build

# Run the container
make docker-run
```

The server will be available at `http://localhost:3000`.

### Manual Installation

1. Clone the repository:
```bash
git clone https://github.com/dannywillems/operating-system.git
cd operating-system
```

2. Install dependencies:
```bash
make install
```

3. Copy the environment file:
```bash
cp .env.example .env
```

4. Run database migrations:
```bash
make migrate
```

5. Start the development server:
```bash
make dev
```

The server will be available at `http://localhost:3000`.

## Usage

### Web Interface

1. Navigate to `http://localhost:3000`
2. Register a new account
3. Create a board and start adding columns and cards

### API

See [docs/api.md](docs/api.md) for complete API documentation.

Quick example:
```bash
# Register
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "password123", "name": "Test User"}'

# Login and get session cookie
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d '{"email": "test@example.com", "password": "password123"}'

# Create a board
curl -X POST http://localhost:3000/api/boards \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{"name": "My Board"}'
```

## Development

### Available Make Targets

```
make help            # Show all available targets
make dev             # Run development server
make test            # Run tests
make build           # Build release binary
make format          # Format code (Rust + TOML)
make format-check    # Check formatting
make lint            # Run clippy
make migrate         # Run database migrations
make db-reset        # Reset the database
make clean           # Remove build artifacts
make check           # Run format-check, lint, and test
make docker-build    # Build Docker image
make docker-run      # Run Docker container
make docker-lint     # Lint Dockerfile with hadolint
```

### Code Quality

Before committing:
```bash
make format
make lint
make test
```

### Project Structure

```
.
|-- Cargo.toml           # Project dependencies
|-- Dockerfile           # Container build instructions
|-- Makefile             # Build automation
|-- migrations/          # Database migrations
|-- src/
|   |-- main.rs          # Application entry point
|   |-- auth/            # Authentication module
|   |-- error.rs         # Error handling
|   |-- handlers/        # HTTP request handlers
|   |-- models/          # Domain models
|   |-- repo/            # Repository layer (database access)
|   |-- state.rs         # Application state
|   `-- static/          # Static assets
|-- templates/           # HTML templates (Askama)
|-- tests/               # Integration tests
`-- docs/                # Documentation
```

## License

MIT
