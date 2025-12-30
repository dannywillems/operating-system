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

## Quick Start

```bash
git clone https://github.com/dannywillems/operating-system.git
cd operating-system
cp .env.example .env
make migrate
make dev
```

The server will be available at `http://localhost:3000`.

## LLM Setup (Ollama)

POS uses a local LLM via [Ollama](https://ollama.ai) for the chat interface.

```bash
# Install Ollama (macOS)
brew install ollama

# Start Ollama server
ollama serve

# Pull the default model
ollama pull llama3.2
```

Configure in `.env` (optional):
```bash
OLLAMA_URL=http://localhost:11434
OLLAMA_MODEL=llama3.2
```

## License

MIT
