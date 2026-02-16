# ══════════════════════════════════════════════════════════
#  Makefile for Slack Incident Bot
# ══════════════════════════════════════════════════════════

.PHONY: help build run test clean docker-build docker-up docker-down migrate fmt lint check dev lean-dev clean-heavy clean-full-local size-report

# ── Default target ──
help:
	@echo "Slack Incident Bot - Available Commands"
	@echo ""
	@echo "Development:"
	@echo "  make dev          - Run in development mode with auto-reload"
	@echo "  make lean-dev     - Run with temporary build artifacts (auto-clean on exit)"
	@echo "  make run          - Build and run the application"
	@echo "  make check        - Check compilation without building"
	@echo "  make test         - Run all tests"
	@echo "  make test-unit    - Run unit tests only"
	@echo "  make fmt          - Format code with rustfmt"
	@echo "  make lint         - Run clippy linter"
	@echo ""
	@echo "Database:"
	@echo "  make migrate      - Run database migrations"
	@echo "  make migrate-new  - Create new migration (usage: make migrate-new NAME=add_users)"
	@echo "  make db-reset     - Drop and recreate database (⚠️  destructive)"
	@echo ""
	@echo "Docker:"
	@echo "  make docker-build - Build Docker image"
	@echo "  make docker-up    - Start services with docker-compose"
	@echo "  make docker-down  - Stop docker-compose services"
	@echo "  make docker-logs  - Follow docker-compose logs"
	@echo ""
	@echo "Build & Release:"
	@echo "  make build        - Build release binary"
	@echo "  make build-dev    - Build debug binary"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make clean-heavy  - Remove heavy build artifacts only"
	@echo "  make clean-full-local - Remove all reproducible local caches/artifacts"
	@echo "  make size-report  - Show disk usage for common heavy paths"
	@echo ""

# ── Build ──
build:
	cargo build --release

build-dev:
	cargo build

# ── Run ──
run: build
	./target/release/incident-bot

dev:
	@command -v cargo-watch >/dev/null 2>&1 || (echo "Installing cargo-watch..." && cargo install cargo-watch)
	cargo watch -x run

lean-dev:
	./scripts/lean-dev.sh

# ── Check & Test ──
check:
	cargo check

test:
	cargo test

test-unit:
	cargo test --lib

test-integration:
	cargo test --test '*'

# ── Code Quality ──
fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

lint:
	cargo clippy -- -D warnings

# ── Database ──
migrate:
	sqlx migrate run

migrate-new:
	@if [ -z "$(NAME)" ]; then \
		echo "Error: NAME is required. Usage: make migrate-new NAME=add_users"; \
		exit 1; \
	fi
	sqlx migrate add $(NAME)

db-reset:
	@echo "⚠️  This will DROP and RECREATE the database. Continue? [y/N] " && read ans && [ $${ans:-N} = y ]
	sqlx database drop -y
	sqlx database create
	sqlx migrate run

# ── Docker ──
docker-build:
	docker build -t incident-bot:latest .

docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-logs:
	docker compose logs -f

docker-restart:
	docker compose restart app

# ── Clean ──
clean:
	cargo clean
	rm -rf target/

clean-heavy:
	./scripts/clean-heavy.sh

clean-full-local:
	./scripts/clean-full-local.sh

size-report:
	@echo "Disk usage snapshot:"
	@for path in target .sqlx "$${TMPDIR:-/tmp}/incident-bot-lean"; do \
		if [ -e "$$path" ]; then \
			du -sh "$$path"; \
		else \
			echo "$$path (missing)"; \
		fi; \
	done

# ── CI ──
ci: fmt-check lint test
	@echo "✅ All CI checks passed"

# ── Installation ──
install-deps:
	@command -v rustup >/dev/null 2>&1 || (echo "Installing rustup..." && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)
	@command -v sqlx >/dev/null 2>&1 || (echo "Installing sqlx-cli..." && cargo install sqlx-cli --no-default-features --features postgres)
	@echo "✅ Dependencies installed"

# ── Release ──
release: clean
	cargo build --release
	strip target/release/incident-bot
	@echo "✅ Release binary: target/release/incident-bot"
