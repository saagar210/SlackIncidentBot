# Slack App Setup Guide

Complete guide to configuring your Slack app for the Incident Bot.

## Prerequisites

- Slack workspace with admin/app installation permissions
- Public URL for the bot (use [ngrok](https://ngrok.com/) for local development)

## Step 1: Create Slack App

1. Navigate to https://api.slack.com/apps
2. Click **"Create New App"**
3. Choose **"From scratch"**
4. Enter:
   - **App Name**: `Incident Bot`
   - **Workspace**: Select your workspace
5. Click **"Create App"**

## Step 2: Configure OAuth Scopes

1. In left sidebar, click **"OAuth & Permissions"**
2. Scroll to **"Scopes"** → **"Bot Token Scopes"**
3. Add the following scopes:

   | Scope | Purpose |
   |-------|---------|
   | `commands` | Register and handle slash commands |
   | `channels:manage` | Create incident channels |
   | `channels:read` | Read channel information |
   | `channels:join` | Join channels to post messages |
   | `chat:write` | Post messages to channels |
   | `pins:write` | Pin incident details |
   | `im:write` | Send DMs for P1 escalations |
   | `users:read` | Look up user information |

## Step 3: Create Slash Command

1. In left sidebar, click **"Slash Commands"**
2. Click **"Create New Command"**
3. Configure:
   - **Command**: `/incident`
   - **Request URL**: `https://your-domain.com/slack/commands`
     - For local dev: `https://your-ngrok-id.ngrok.io/slack/commands`
   - **Short Description**: `Manage incidents`
   - **Usage Hint**: `declare | status | severity | resolved | timeline | postmortem`
4. Click **"Save"**

## Step 4: Enable Interactivity

1. In left sidebar, click **"Interactivity & Shortcuts"**
2. Toggle **"Interactivity"** to **On**
3. Set **Request URL**: `https://your-domain.com/slack/interactions`
   - For local dev: `https://your-ngrok-id.ngrok.io/slack/interactions`
4. Click **"Save Changes"**

## Step 5: Install App to Workspace

1. In left sidebar, click **"Install App"**
2. Click **"Install to Workspace"**
3. Review permissions and click **"Allow"**
4. You'll see a **Bot User OAuth Token** starting with `xoxb-`
5. Copy this token (you'll need it for `.env`)

## Step 6: Get Signing Secret

1. In left sidebar, click **"Basic Information"**
2. Scroll to **"App Credentials"**
3. Under **"Signing Secret"**, click **"Show"**
4. Copy the secret (you'll need it for `.env`)

## Step 7: Configure Environment Variables

Copy the tokens to your `.env` file:

```bash
SLACK_BOT_TOKEN=xoxb-YOUR-BOT-TOKEN-HERE
SLACK_SIGNING_SECRET=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
```

## Step 8: Get Channel and User IDs

### Finding Channel IDs

1. Open Slack desktop/web (not mobile)
2. Navigate to the channel (e.g., #general)
3. Right-click the channel name → **"Copy link"**
4. The URL looks like: `https://yourworkspace.slack.com/archives/C024BE91L`
5. The ID is the last part: `C024BE91L`

Add to `.env`:
```bash
NOTIFICATION_CHANNEL_GENERAL=C024BE91L
NOTIFICATION_CHANNEL_ENGINEERING=C024BE92M
```

### Finding User IDs

1. Open Slack desktop/web
2. Click on a user's profile
3. Click **"More"** → **"Copy member ID"**
4. The ID looks like: `U024BE7LH`

Add to `.env`:
```bash
EXEC_NOTIFICATION_USERS=U024BE7LH,U024BE7LJ,U024BE7LK
```

## Step 9: Test the Integration

### Start the Bot

```bash
# If using ngrok for local dev:
ngrok http 3000

# In another terminal:
cargo run
```

### Test in Slack

1. In any Slack channel, type: `/incident`
2. You should see autocomplete showing: `/incident declare`
3. Run: `/incident declare`
4. A modal should appear with incident creation form
5. Fill it out and submit
6. Check logs for successful incident creation

## Troubleshooting

### "Slash command not found"

**Cause**: Slack hasn't registered the command yet
**Fix**:
- Wait 5 minutes for Slack to propagate changes
- Or reinstall the app (OAuth & Permissions → Reinstall)

### "Request verification failed"

**Cause**: Signing secret mismatch or timestamp issues
**Fix**:
- Verify `SLACK_SIGNING_SECRET` in `.env` matches Slack app
- Check system clock is synchronized (Slack requires ±5 minute accuracy)
- Enable debug logs: `RUST_LOG=incident_bot=debug`

### "Bot not responding to commands"

**Cause**: Request URL not reachable
**Fix**:
- Verify ngrok is running: `curl https://your-ngrok-id.ngrok.io/health`
- Check bot logs for incoming requests
- Verify firewall allows incoming HTTPS

### "Channel creation fails"

**Cause**: Missing `channels:manage` scope
**Fix**:
- Add scope in OAuth & Permissions
- Reinstall app to workspace

### "Cannot pin messages"

**Cause**: Missing `pins:write` scope
**Fix**:
- Add scope in OAuth & Permissions
- Reinstall app to workspace

## Local Development Tips

### Using ngrok

```bash
# Install ngrok
brew install ngrok  # macOS
# or download from https://ngrok.com/download

# Start tunnel (in separate terminal)
ngrok http 3000

# Copy the HTTPS URL (e.g., https://abc123.ngrok.io)
# Update Slack app's Request URLs with this URL
```

### Testing Without Reinstalling

- For code changes, just restart the bot (no Slack config change needed)
- For Slack config changes (scopes, URLs), reinstall the app
- Use `RUST_LOG=incident_bot=debug` to see all Slack requests

### Webhook Debugging

Enable request logging to see raw Slack payloads:

```bash
# In main.rs, add TraceLayer to see all HTTP requests
RUST_LOG=incident_bot=debug,tower_http=debug cargo run
```

## Production Considerations

### Request URLs

Replace ngrok URLs with permanent production URLs:
- `https://incidents.yourcompany.com/slack/commands`
- `https://incidents.yourcompany.com/slack/interactions`

### Security

- Keep `SLACK_BOT_TOKEN` and `SLACK_SIGNING_SECRET` secret
- Use environment variables or secret management (not hardcoded)
- Enable HTTPS in production (required by Slack)
- Consider IP allowlisting if your infrastructure supports it

### Bot Display

Optional customization in **"Basic Information"**:
- **App icon**: Upload 512x512 icon
- **Background color**: Brand color for bot messages
- **Description**: "Incident management orchestration for [Company]"

## Next Steps

Once Slack is configured:
1. Complete `.env` configuration (see [CONFIGURATION.md](./CONFIGURATION.md))
2. Start the bot and test incident lifecycle
3. Configure service owners and notification channels
4. (Optional) Set up Statuspage integration
5. Deploy to production (see [DEPLOYMENT.md](./DEPLOYMENT.md))
