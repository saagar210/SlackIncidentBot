# Configuration Reference

Complete reference for all environment variables used by the Incident Bot.

## Quick Start

```bash
# Copy template and edit
cp .env.example .env
nano .env
```

## Required Configuration

These variables **must** be set for the bot to function.

### `DATABASE_URL`

PostgreSQL connection string.

**Format**: `postgres://user:password@host:port/database`

**Example**:
```bash
DATABASE_URL=postgres://incident_bot:securepassword@localhost:5432/incident_bot
```

**Notes**:
- Use connection pooling-compatible URL
- Supports SSL with `?sslmode=require` parameter
- Bot creates connection pool with 5 connections by default

---

### `SLACK_BOT_TOKEN`

OAuth bot token from Slack app configuration.

**Format**: `xoxb-` followed by token string

**Example**:
```bash
SLACK_BOT_TOKEN=xoxb-YOUR-BOT-TOKEN-HERE
```

**Where to find**:
1. Go to https://api.slack.com/apps
2. Select your app
3. **OAuth & Permissions** → **Bot User OAuth Token**

**Notes**:
- Keep this secret (never commit to git)
- Rotate if compromised
- Required scopes: `commands`, `channels:manage`, `channels:read`, `chat:write`, `pins:write`, `im:write`, `users:read`

---

### `SLACK_SIGNING_SECRET`

Secret for verifying Slack request authenticity.

**Format**: 32-character hexadecimal string

**Example**:
```bash
SLACK_SIGNING_SECRET=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
```

**Where to find**:
1. Go to https://api.slack.com/apps
2. Select your app
3. **Basic Information** → **App Credentials** → **Signing Secret**

**Notes**:
- Used for HMAC-SHA256 signature verification
- Prevents request forgery attacks
- Never expose publicly

---

### `SERVICES`

Comma-separated list of services that can have incidents.

**Format**: `service1,service2,service3`

**Example**:
```bash
SERVICES=auth-service,api-gateway,payment-processor,database,frontend,vpn
```

**Notes**:
- Displayed in incident declaration modal dropdown
- Service names used for channel naming: `inc-YYYYMMDD-service-name`
- Can include spaces, but hyphenated names recommended
- Case-sensitive

---

## Optional Configuration

These variables are optional but enable additional features.

### Server Configuration

#### `PORT`

HTTP server port.

**Default**: `3000`

**Example**:
```bash
PORT=8080
```

---

### Service Ownership

#### `SERVICE_OWNERS`

Maps services to Slack user IDs for auto-invitations.

**Format**: JSON object mapping service names to user ID arrays

**Example**:
```bash
SERVICE_OWNERS={"auth-service":["U024BE7LH","U024BE7LJ"],"api-gateway":["U024BE7LH"],"database":["U024BE7LK"]}
```

**Notes**:
- When incident declared for a service, owners auto-invited to channel
- Must be valid JSON (use double quotes)
- User IDs found via Slack profile → More → Copy member ID
- If not set, only commander invited to channel

---

### Notification Channels

#### `NOTIFICATION_CHANNEL_GENERAL`

Slack channel ID for P1 incident broadcasts.

**Format**: Channel ID starting with `C`

**Example**:
```bash
NOTIFICATION_CHANNEL_GENERAL=C024BE91L
```

**Notes**:
- P1 incidents posted here automatically
- Usually set to #general or #incidents
- Find ID: Right-click channel → Copy link → Extract ID from URL

---

#### `NOTIFICATION_CHANNEL_ENGINEERING`

Slack channel ID for P2 incident notifications.

**Format**: Channel ID starting with `C`

**Example**:
```bash
NOTIFICATION_CHANNEL_ENGINEERING=C024BE92M
```

**Notes**:
- P2 incidents posted here automatically
- Usually set to #engineering or #on-call
- P3/P4 incidents only notify the incident channel (not broadcast)

---

#### `P1_USERS`

Comma-separated Slack user IDs for P1 DMs.

**Format**: `USER_ID1,USER_ID2,USER_ID3`

**Example**:
```bash
P1_USERS=U024BE7LH,U024BE7LJ,U024BE7LK
```

**Notes**:
- These users receive DMs for every P1 incident
- Usually C-suite, VPs, or on-call managers
- Notifications throttled (5-minute window prevents duplicates)

---

### Statuspage Integration

#### `STATUSPAGE_API_KEY`

Statuspage.io OAuth token.

**Format**: OAuth token string

**Example**:
```bash
STATUSPAGE_API_KEY=your-oauth-token-here
```

**Where to find**:
1. Go to https://manage.statuspage.io
2. Select your page
3. **API Info** → Generate new token

**Notes**:
- Optional: Leave empty to disable Statuspage integration
- Requires `STATUSPAGE_PAGE_ID` to be set as well
- Token should have component update permissions

---

#### `STATUSPAGE_PAGE_ID`

Statuspage.io page identifier.

**Format**: Page ID string

**Example**:
```bash
STATUSPAGE_PAGE_ID=abc123xyz
```

**Where to find**:
- Extract from Statuspage management URL: `https://manage.statuspage.io/pages/{PAGE_ID}`

**Notes**:
- Optional: Leave empty to disable Statuspage integration
- Requires `STATUSPAGE_API_KEY` to be set as well
- After enabling, map services to components in database:
  ```sql
  INSERT INTO statuspage_mappings (service_name, component_id)
  VALUES ('api-gateway', 'component-id-from-statuspage');
  ```

---

### Logging

#### `RUST_LOG`

Controls log verbosity using env_logger syntax.

**Default**: `incident_bot=debug,tower_http=debug,axum=debug`

**Example**:
```bash
# Production (less verbose)
RUST_LOG=incident_bot=info,tower_http=info,axum=warn

# Development (very verbose)
RUST_LOG=incident_bot=trace,tower_http=trace,axum=trace,sqlx=debug

# Minimal (errors only)
RUST_LOG=incident_bot=error
```

**Log Levels** (from most to least verbose):
- `trace` - Every function entry/exit
- `debug` - Detailed debugging information
- `info` - General informational messages
- `warn` - Warning messages
- `error` - Error messages only

**Notes**:
- Use `info` or `warn` in production
- Use `debug` or `trace` for troubleshooting
- Logs output as JSON for structured logging

---

## Configuration Validation

The bot validates configuration on startup:

```bash
cargo run
# Logs will show validation results
```

**Common validation errors**:

| Error | Cause | Fix |
|-------|-------|-----|
| `SLACK_BOT_TOKEN must start with xoxb-` | Invalid token format | Copy token from Slack app config |
| `SERVICES cannot be empty` | No services configured | Add at least one service |
| `Invalid JSON in SERVICE_OWNERS` | Malformed JSON | Use valid JSON with double quotes |
| `Database connection failed` | Bad DATABASE_URL | Verify PostgreSQL is running |

---

## Environment Files

### Development: `.env`

- Local development configuration
- **Never commit to git** (included in `.gitignore`)
- Copy from `.env.example` and customize

### Production

**Option 1: Environment variables** (recommended)
```bash
export DATABASE_URL="postgres://..."
export SLACK_BOT_TOKEN="xoxb-..."
./incident-bot
```

**Option 2: `.env` file**
```bash
./incident-bot --env-file /etc/incident-bot/.env
```

**Option 3: Secret management**
- AWS Secrets Manager
- HashiCorp Vault
- Kubernetes Secrets

---

## Security Best Practices

1. **Never commit secrets**
   - Add `.env` to `.gitignore`
   - Use `.env.example` for templates only

2. **Restrict access**
   - File permissions: `chmod 600 .env`
   - Only bot process user should read

3. **Rotate credentials**
   - Rotate Slack tokens if compromised
   - Use different tokens per environment (dev/staging/prod)

4. **Audit configuration**
   - Review who has access to secret storage
   - Log configuration changes

---

## Example Configurations

### Minimal (Required Only)

```bash
DATABASE_URL=postgres://user:pass@localhost/incident_bot
SLACK_BOT_TOKEN=xoxb-your-token
SLACK_SIGNING_SECRET=your-secret
SERVICES=frontend,backend,database
```

### Recommended (With Notifications)

```bash
DATABASE_URL=postgres://user:pass@localhost/incident_bot
SLACK_BOT_TOKEN=xoxb-your-token
SLACK_SIGNING_SECRET=your-secret
SERVICES=auth,api,payments,database,frontend

SERVICE_OWNERS={"auth":["U024USER1"],"api":["U024USER2"],"payments":["U024USER3"]}
NOTIFICATION_CHANNEL_GENERAL=C024GENERAL
NOTIFICATION_CHANNEL_ENGINEERING=C024ENGINEERING
P1_USERS=U024CEO,U024CTO,U024VPENG

RUST_LOG=incident_bot=info,tower_http=info
```

### Full (All Features Enabled)

```bash
# Server
PORT=3000

# Database
DATABASE_URL=postgres://incident_bot:securepass@db.internal:5432/incident_bot

# Slack
SLACK_BOT_TOKEN=xoxb-YOUR-BOT-TOKEN-HERE
SLACK_SIGNING_SECRET=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6

# Services
SERVICES=auth-service,api-gateway,payment-processor,database,frontend,cdn

# Service ownership
SERVICE_OWNERS={"auth-service":["U024AUTH1","U024AUTH2"],"api-gateway":["U024API1"],"payment-processor":["U024PAY1","U024PAY2"],"database":["U024DBA1"],"frontend":["U024FE1"],"cdn":["U024INFRA1"]}

# Notifications
NOTIFICATION_CHANNEL_GENERAL=C024GENERAL
NOTIFICATION_CHANNEL_ENGINEERING=C024ENGINEERING
P1_USERS=U024CEO,U024CTO,U024VPENG,U024ONCALL

# Statuspage
STATUSPAGE_API_KEY=sp_oauth_1234567890abcdef
STATUSPAGE_PAGE_ID=abc123xyz

# Logging
RUST_LOG=incident_bot=info,tower_http=info,axum=warn
```

---

## Troubleshooting

### Configuration not loading

**Symptoms**: Bot starts but ignores `.env` file

**Fix**: Ensure `.env` is in working directory or use explicit path:
```bash
# Current directory
cargo run

# Explicit path
DATABASE_URL=... SLACK_BOT_TOKEN=... cargo run
```

### JSON parse errors in SERVICE_OWNERS

**Symptoms**: `Invalid JSON in SERVICE_OWNERS`

**Fix**: Use valid JSON syntax:
```bash
# ✅ Correct (double quotes)
SERVICE_OWNERS={"service":["U024USER"]}

# ❌ Wrong (single quotes)
SERVICE_OWNERS={'service':['U024USER']}

# ❌ Wrong (trailing comma)
SERVICE_OWNERS={"service":["U024USER"],}
```

### Statuspage not syncing

**Symptoms**: Incidents declared but Statuspage unchanged

**Fix**:
1. Verify both `STATUSPAGE_API_KEY` and `STATUSPAGE_PAGE_ID` are set
2. Check database for component mappings:
   ```sql
   SELECT * FROM statuspage_mappings;
   ```
3. Add mapping if missing:
   ```sql
   INSERT INTO statuspage_mappings (service_name, component_id)
   VALUES ('your-service', 'statuspage-component-id');
   ```
4. Check logs for Statuspage API errors: `RUST_LOG=incident_bot=debug`

---

## Next Steps

After configuration:
1. Run database migrations: `sqlx migrate run`
2. Start the bot: `cargo run`
3. Test in Slack: `/incident declare`
4. Monitor logs for any configuration warnings
