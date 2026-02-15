# Quick Start Guide

## Prerequisites

- Rust 1.70+ installed
- PostgreSQL 16 running (via Docker or local)
- Slack workspace with admin access

## 1. Set Up Database

```bash
# Start PostgreSQL with Docker
docker run -d \
  --name incident-bot-db \
  -e POSTGRES_USER=incident_bot \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=incident_bot \
  -p 5432:5432 \
  postgres:16-alpine
```

## 2. Configure Slack App

1. Go to https://api.slack.com/apps
2. Click "Create New App" → "From scratch"
3. Name: "Incident Bot", choose your workspace
4. **OAuth & Permissions**:
   - Add scopes:
     - `commands`
     - `channels:manage`
     - `channels:read`
     - `chat:write`
     - `pins:write`
     - `im:write`
     - `users:read`
   - Install app to workspace
   - Copy **Bot User OAuth Token** (starts with `xoxb-`)

5. **Slash Commands**:
   - Create command: `/incident`
   - Request URL: `https://your-ngrok-url/slack/commands`
   - Description: "Manage incidents"
   - Usage hint: "[declare|status|severity|resolved|timeline|postmortem]"

6. **Interactivity**:
   - Enable Interactivity
   - Request URL: `https://your-ngrok-url/slack/interactions`

7. **Basic Information**:
   - Copy **Signing Secret** (under App Credentials)

## 3. Set Up Ngrok (for local development)

```bash
# Install ngrok (if not installed)
brew install ngrok

# Start ngrok
ngrok http 3000
```

Copy the HTTPS URL (e.g., `https://abc123.ngrok.io`) and update your Slack app's URLs.

## 4. Configure Environment

```bash
# Copy example env file
cp .env.example .env

# Edit .env with your credentials
nano .env
```

Update these values:
```bash
SLACK_BOT_TOKEN=xoxb-your-token-here
SLACK_SIGNING_SECRET=your-signing-secret
DATABASE_URL=postgres://incident_bot:password@localhost:5432/incident_bot

# Notification routing (Slack user IDs)
P1_DM_RECIPIENTS=U024YOURUSERID  # Get from Slack profile → More → Copy member ID
P1_CHANNELS=C024GENERALID        # Get from channel → View channel details → Copy channel ID
P2_CHANNELS=C024ENGINEERINGID

# Available services (comma-separated)
SERVICES=Okta,VPN,Email,SSO,Database,API Gateway

# Service owners (JSON)
SERVICE_OWNERS={"Okta":["U024YOURUSERID"],"VPN":["U024YOURUSERID"]}
```

## 5. Run Migrations

```bash
# Install sqlx CLI
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run
```

## 6. Start the Bot

```bash
cargo run
```

You should see:
```
Starting Incident Bot
Configuration loaded
Database connection pool created
Running database migrations
Database migrations complete
Listening on 0.0.0.0:3000
```

## 7. Test in Slack

### Declare an Incident

In any Slack channel:
```
/incident declare
```

A modal should appear. Fill it in:
- **Title**: Test incident - Okta SSO outage
- **Severity**: P2 (High)
- **Service**: Okta
- **Commander**: (your username, pre-filled)

Click **Declare**.

The bot will:
- Create channel `#inc-20260215-okta`
- Invite you to the channel
- Pin incident details
- Post to #engineering (P2 channel)

### Post Status Updates

In the incident channel:
```
/incident status Investigating root cause
```

### Change Severity

```
/incident severity P1 Impact increased to 200+ users
```

This will escalate and post to #general + DM you.

### Resolve Incident

```
/incident resolved
```

### View Timeline

```
/incident timeline
```

### Generate Postmortem

```
/incident postmortem
```

## Common Issues

### "Command failed with error: 'invalid signature'"

- Check your `SLACK_SIGNING_SECRET` is correct
- Make sure ngrok is running and URL matches Slack app config

### "Database connection failed"

- Check PostgreSQL is running: `docker ps`
- Check `DATABASE_URL` in `.env`
- Try: `psql postgres://incident_bot:password@localhost:5432/incident_bot`

### "Channel creation failed"

- Check bot has `channels:manage` scope
- Reinstall app to workspace after adding scopes

### "Permission denied" when running commands

- Make sure you're the incident commander
- Only the commander can post status/change severity/resolve

### Bot doesn't respond

- Check bot logs: `cargo run` output
- Check ngrok is running: `curl http://localhost:4040/api/tunnels`
- Check Slack app URLs match ngrok URL
- Test signature verification: should see log "Received slash command: /incident ..."

## Development Tips

### Watch mode (auto-restart on changes)

```bash
cargo install cargo-watch
cargo watch -x run
```

### Check compilation

```bash
cargo check
```

### Format code

```bash
cargo fmt
```

### Lint

```bash
cargo clippy
```

### View database

```bash
psql postgres://incident_bot:password@localhost:5432/incident_bot

# List incidents
SELECT id, title, severity, status, commander_id FROM incidents;

# View timeline for an incident
SELECT event_type, message, posted_by, timestamp
FROM incident_timeline
WHERE incident_id = 'incident-uuid-here'
ORDER BY timestamp;
```

## Next Steps

- Read `PHASE1_COMPLETE.md` for full feature documentation
- Set up multiple test incidents to validate full lifecycle
- Configure your team's actual P1/P2 recipient lists
- Add your real services to `SERVICES` env var
- Map service owners in `SERVICE_OWNERS`

## Support

Check logs for errors. Most issues are:
1. Slack app configuration (scopes, URLs)
2. Environment variables (missing or incorrect)
3. Database connection (PostgreSQL not running)

All errors are logged with structured JSON output for easy debugging.
