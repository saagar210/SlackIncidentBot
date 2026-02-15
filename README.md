# Slack Incident Bot

A production-ready Slack bot for orchestrating incident lifecycle management, built in Rust.

## Features

✅ **Complete Incident Lifecycle**
- Declare incidents with severity levels (P1-P4)
- Automatic channel creation and team notifications
- Status updates with timeline tracking
- Severity escalation with re-notifications
- Incident resolution with duration tracking
- Post-mortem generation

✅ **Intelligent Notifications**
- P1: Broadcast to #general + DM executives
- P2: Post to #engineering
- P3/P4: Channel-only notifications
- Duplicate notification throttling (5-minute window)

✅ **Statuspage Integration**
- Automatic component status updates
- Severity-aware status mapping
- Async job queue for reliability
- Graceful degradation if unavailable

✅ **Production Ready**
- Slack signature verification
- Commander-only permissions for critical operations
- PostgreSQL with compile-time query validation
- Comprehensive error handling
- Full audit trail

## Quick Start

### Prerequisites

- Rust 1.70+ ([install rustup](https://rustup.rs/))
- PostgreSQL 16+
- Slack workspace with admin access

### Installation

```bash
# 1. Clone and navigate to project
cd /path/to/SlackIncidentBot

# 2. Start PostgreSQL
docker compose up -d

# 3. Configure environment
cp .env.example .env
# Edit .env with your Slack credentials (see CONFIGURATION.md)

# 4. Run migrations
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run

# 5. Build and run
cargo run --release

# 6. Verify health
curl http://localhost:3000/health
```

### Slack App Setup

See [SLACK_SETUP.md](./SLACK_SETUP.md) for detailed Slack app configuration.

**Quick version:**
1. Create app at https://api.slack.com/apps
2. Add OAuth scopes: `commands`, `channels:manage`, `channels:read`, `chat:write`, `pins:write`, `im:write`, `users:read`
3. Create slash command `/incident` → `https://your-url/slack/commands`
4. Enable interactivity → `https://your-url/slack/interactions`
5. Install to workspace
6. Copy bot token and signing secret to `.env`

## Usage

### Declaring an Incident

```
/incident declare
```

Opens a modal to capture:
- **Title**: Brief description (e.g., "API Gateway returning 500s")
- **Severity**: P1 (Critical) through P4 (Low)
- **Service**: Affected service from configured list
- **Commander**: Incident commander (defaults to you)

Creates:
- Dedicated incident channel (`inc-YYYYMMDD-service-name`)
- Pinned incident details
- Timeline entry
- Severity-based notifications

### Managing an Incident

All commands must be run in the incident channel:

```bash
# Update status
/incident status Identified root cause in load balancer config

# Change severity (triggers re-notifications if escalating to P1/P2)
/incident severity P1 Database is completely down

# View timeline
/incident timeline

# Mark resolved
/incident resolved

# Generate post-mortem template
/incident postmortem
```

### Permissions

- **Anyone** can declare incidents
- **Commander only** can:
  - Post status updates
  - Change severity
  - Resolve incidents
  - Generate post-mortems

## Architecture

### Tech Stack

- **Runtime**: Rust + Tokio async runtime
- **Web Framework**: Axum
- **Database**: PostgreSQL with sqlx (compile-time query checking)
- **Slack API**: Custom reqwest-based client (no SDK dependencies)
- **Job Queue**: In-process tokio::mpsc channels

### Key Design Decisions

1. **No Slack SDK** - Built on raw HTTP + typed structs for full control and maintainability
2. **Ack-then-Process** - Return 200 OK immediately, spawn async tasks for 3-second compliance
3. **State Machine** - Explicit state transitions: Declared → Investigating → Identified → Monitoring → Resolved
4. **Best-Effort External APIs** - Statuspage sync failures logged but don't block incident workflow
5. **In-Process Queue** - Simple tokio channels for MVP (can swap for Redis later)

### Project Structure

```
src/
├── main.rs                  # Server, routes, startup
├── app_state.rs             # Shared state (DB pool, Slack client, config)
├── config.rs                # Environment variable configuration
├── error.rs                 # Custom error types with Axum integration
│
├── commands/                # Slash command handlers
│   ├── declare.rs           # /incident declare
│   ├── status.rs            # /incident status
│   ├── severity.rs          # /incident severity
│   ├── resolved.rs          # /incident resolved
│   ├── timeline.rs          # /incident timeline
│   └── postmortem.rs        # /incident postmortem
│
├── services/                # Business logic layer
│   ├── incident.rs          # State machine, CRUD operations
│   ├── notification.rs      # Severity-based routing
│   ├── timeline.rs          # Timeline event tracking
│   ├── postmortem.rs        # Template generation
│   └── audit.rs             # Audit logging
│
├── slack/                   # Slack API integration
│   ├── client.rs            # HTTP client wrapper
│   ├── verification.rs      # HMAC-SHA256 signature verification
│   ├── events.rs            # Request parsing
│   ├── blocks.rs            # Block Kit message builders
│   └── modals.rs            # Modal definitions
│
├── db/                      # Data layer
│   ├── mod.rs               # Pool setup, migrations
│   ├── models.rs            # Rust types (Incident, Severity, etc.)
│   └── queries/             # Database query functions
│
├── adapters/                # External API integrations
│   └── statuspage.rs        # Statuspage.io client
│
├── jobs/                    # Async background jobs
│   ├── mod.rs               # Job enum
│   ├── worker.rs            # Background worker
│   └── statuspage_sync.rs   # Statuspage sync job
│
└── utils/                   # Shared utilities
    └── channel.rs           # Channel naming logic
```

## Database Schema

See `migrations/20260215000001_initial_schema.sql` for full schema.

**Core tables:**
- `incidents` - Incident metadata and current state
- `incident_timeline` - Immutable event log
- `incident_notifications` - Notification delivery audit
- `statuspage_mappings` - Service → Statuspage component mapping
- `audit_log` - Every command and state change

## Development

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Watch mode (requires cargo-watch)
cargo install cargo-watch
cargo watch -x run

# Database migrations
sqlx migrate add <name>
sqlx migrate run
```

## Deployment

See [DEPLOYMENT.md](./DEPLOYMENT.md) for production deployment guide.

**Docker:**
```bash
docker build -t incident-bot .
docker run -p 3000:3000 --env-file .env incident-bot
```

**Health checks:**
- `GET /health` - Returns "OK" if database is reachable

## Configuration

See [CONFIGURATION.md](./CONFIGURATION.md) for complete environment variable reference.

**Required variables:**
```bash
DATABASE_URL=postgres://user:pass@localhost/db
SLACK_BOT_TOKEN=xoxb-...
SLACK_SIGNING_SECRET=...
SERVICES=service1,service2,service3
```

**Optional but recommended:**
```bash
SERVICE_OWNERS={"service1":["U024USER1"]}
NOTIFICATION_CHANNEL_GENERAL=C024CHANNEL
NOTIFICATION_CHANNEL_ENGINEERING=C024CHANNEL
EXEC_NOTIFICATION_USERS=U024USER1,U024USER2
```

## Testing

18/18 unit tests passing. See [TESTING.md](./TESTING.md) for test coverage.

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests (requires database)
cargo test --test '*'
```

## Statuspage Integration

Optional integration with Statuspage.io for public status page updates.

**Setup:**
1. Add API credentials to `.env`:
   ```bash
   STATUSPAGE_API_KEY=your-oauth-token
   STATUSPAGE_PAGE_ID=your-page-id
   ```

2. Map services to components:
   ```sql
   INSERT INTO statuspage_mappings (service_name, component_id)
   VALUES ('api-gateway', 'abcd1234');
   ```

3. Status updates happen automatically on incident state changes

**Status Mapping:**
- P1 Declared/Investigating → `major_outage`
- P2 Declared/Investigating → `partial_outage`
- P1 Identified/Monitoring → `partial_outage`
- All others → `degraded_performance`
- Resolved → `operational`

## Troubleshooting

**Bot not responding:**
- Check `RUST_LOG=incident_bot=debug` for detailed logs
- Verify Slack signing secret matches
- Ensure request URL is publicly accessible (use ngrok for local dev)

**Database connection errors:**
- Verify PostgreSQL is running: `docker compose ps`
- Check DATABASE_URL format
- Run migrations: `sqlx migrate run`

**Channel creation fails:**
- Verify bot has `channels:manage` scope
- Check bot is installed to workspace

## Contributing

This is an internal tool. For bugs or feature requests, create an issue or submit a PR.

**Development workflow:**
1. Create feature branch
2. Write tests
3. Ensure `cargo test` and `cargo clippy` pass
4. Submit PR with conventional commits (`feat:`, `fix:`, `docs:`)

## License

Internal company tool - not licensed for external use.
