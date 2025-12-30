# Personal Operating System

A Kanban-based task management webapp built with Rust, featuring boards, columns, cards, tags, and a full REST API.

## Features

- Kanban board with drag-and-drop-ready cards
- Multiple boards with customizable columns
- Card tagging and filtering
- Full-text search on cards
- Date filtering (start, end, due dates)
- User authentication with email/password
- Session-based auth for web, API tokens for programmatic access
- Board permissions: owner, editor, reader
- Card visibility: private, restricted, public
- Server-rendered UI with Bootstrap
- REST API with JSON responses

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

### Installation

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
