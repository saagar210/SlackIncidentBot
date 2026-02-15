# Deployment Guide

Production deployment guide for the Slack Incident Bot.

## Pre-Deployment Checklist

- [ ] Slack app created and configured (see [SLACK_SETUP.md](./SLACK_SETUP.md))
- [ ] All tests passing (`cargo test`)
- [ ] Configuration reviewed (see [CONFIGURATION.md](./CONFIGURATION.md))
- [ ] PostgreSQL instance provisioned
- [ ] Domain/subdomain ready (e.g., `incidents.yourcompany.com`)
- [ ] SSL certificate provisioned
- [ ] Secrets management strategy chosen

## Deployment Options

### Option 1: Docker (Recommended)

Best for: Kubernetes, ECS, Cloud Run, or standalone Docker hosts

### Option 2: Binary Deploy

Best for: VMs, bare metal, or serverless containers (Fargate, etc.)

### Option 3: Platform-as-a-Service

Best for: Fly.io, Railway, Render

---

## Option 1: Docker Deployment

### Build Docker Image

```bash
# Build multi-stage production image
docker build -t incident-bot:latest .

# Tag for registry
docker tag incident-bot:latest your-registry.com/incident-bot:v1.0.0

# Push to registry
docker push your-registry.com/incident-bot:v1.0.0
```

### Run Locally with Docker

```bash
# Using .env file
docker run -d \
  --name incident-bot \
  -p 3000:3000 \
  --env-file .env \
  incident-bot:latest

# Or with environment variables
docker run -d \
  --name incident-bot \
  -p 3000:3000 \
  -e DATABASE_URL="postgres://..." \
  -e SLACK_BOT_TOKEN="xoxb-..." \
  -e SLACK_SIGNING_SECRET="..." \
  -e SERVICES="api,db,frontend" \
  incident-bot:latest

# Check logs
docker logs -f incident-bot

# Health check
curl http://localhost:3000/health
```

### Docker Compose (Development)

```bash
# Start full stack (app + database)
docker compose up -d

# View logs
docker compose logs -f app

# Restart after code changes
docker compose up -d --build app

# Teardown
docker compose down
```

---

## Option 2: Binary Deployment

### Build Release Binary

```bash
# Build optimized release binary
cargo build --release

# Binary located at:
# target/release/incident-bot

# Test locally
DATABASE_URL="postgres://..." \
SLACK_BOT_TOKEN="xoxb-..." \
./target/release/incident-bot
```

### Deploy to Server

```bash
# On your deployment server
mkdir -p /opt/incident-bot
cd /opt/incident-bot

# Copy binary (from build machine)
scp user@build-server:/path/to/target/release/incident-bot ./

# Make executable
chmod +x incident-bot

# Create .env file
cp .env.example .env
nano .env  # Edit with production values

# Run migrations
# (Requires sqlx-cli on server or run from build machine)
sqlx migrate run --database-url "$DATABASE_URL"

# Test
./incident-bot
```

### Systemd Service (Linux)

Create `/etc/systemd/system/incident-bot.service`:

```ini
[Unit]
Description=Slack Incident Bot
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=incident-bot
Group=incident-bot
WorkingDirectory=/opt/incident-bot
EnvironmentFile=/opt/incident-bot/.env
ExecStart=/opt/incident-bot/incident-bot
Restart=always
RestartSec=10

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=incident-bot

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/incident-bot

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
# Create user
sudo useradd -r -s /bin/false incident-bot

# Set permissions
sudo chown -R incident-bot:incident-bot /opt/incident-bot
sudo chmod 600 /opt/incident-bot/.env

# Enable service
sudo systemctl enable incident-bot
sudo systemctl start incident-bot

# Check status
sudo systemctl status incident-bot

# View logs
sudo journalctl -u incident-bot -f
```

---

## Option 3: Platform-as-a-Service

### Fly.io

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Login
fly auth login

# Create app
fly launch

# Set secrets
fly secrets set \
  SLACK_BOT_TOKEN="xoxb-..." \
  SLACK_SIGNING_SECRET="..." \
  DATABASE_URL="postgres://..."

# Deploy
fly deploy

# Scale
fly scale count 2  # Run 2 instances
fly scale vm shared-cpu-2x  # Larger VM

# Logs
fly logs
```

### Railway

```bash
# Install Railway CLI
npm install -g @railway/cli

# Login
railway login

# Create project
railway init

# Set environment variables
railway variables set DATABASE_URL="postgres://..."
railway variables set SLACK_BOT_TOKEN="xoxb-..."

# Deploy
railway up

# Logs
railway logs
```

### Render

1. Connect GitHub repository
2. Create new **Web Service**
3. Build command: `cargo build --release`
4. Start command: `./target/release/incident-bot`
5. Add environment variables in dashboard
6. Deploy

---

## Kubernetes Deployment

### Create Kubernetes Manifests

**`k8s/namespace.yaml`**:
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: incident-bot
```

**`k8s/secret.yaml`**:
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: incident-bot-secrets
  namespace: incident-bot
type: Opaque
stringData:
  slack-bot-token: "xoxb-your-token"
  slack-signing-secret: "your-secret"
  database-url: "postgres://user:pass@postgres:5432/incident_bot"
```

**`k8s/configmap.yaml`**:
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: incident-bot-config
  namespace: incident-bot
data:
  SERVICES: "auth,api,payments,database"
  SERVICE_OWNERS: '{"auth":["U024USER1"],"api":["U024USER2"]}'
  NOTIFICATION_CHANNEL_GENERAL: "C024GENERAL"
  NOTIFICATION_CHANNEL_ENGINEERING: "C024ENGINEERING"
  RUST_LOG: "incident_bot=info,tower_http=info"
```

**`k8s/deployment.yaml`**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: incident-bot
  namespace: incident-bot
spec:
  replicas: 2
  selector:
    matchLabels:
      app: incident-bot
  template:
    metadata:
      labels:
        app: incident-bot
    spec:
      containers:
      - name: incident-bot
        image: your-registry.com/incident-bot:v1.0.0
        ports:
        - containerPort: 3000
        env:
        - name: PORT
          value: "3000"
        envFrom:
        - configMapRef:
            name: incident-bot-config
        - secretRef:
            name: incident-bot-secrets
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 10
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

**`k8s/service.yaml`**:
```yaml
apiVersion: v1
kind: Service
metadata:
  name: incident-bot
  namespace: incident-bot
spec:
  selector:
    app: incident-bot
  ports:
  - port: 80
    targetPort: 3000
  type: ClusterIP
```

**`k8s/ingress.yaml`** (nginx-ingress example):
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: incident-bot
  namespace: incident-bot
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  ingressClassName: nginx
  tls:
  - hosts:
    - incidents.yourcompany.com
    secretName: incident-bot-tls
  rules:
  - host: incidents.yourcompany.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: incident-bot
            port:
              number: 80
```

### Deploy to Kubernetes

```bash
# Apply all manifests
kubectl apply -f k8s/

# Check deployment
kubectl -n incident-bot get pods
kubectl -n incident-bot logs -f deployment/incident-bot

# Check service
kubectl -n incident-bot get svc
kubectl -n incident-bot get ingress
```

---

## Database Setup

### PostgreSQL (Managed Services)

**AWS RDS**:
1. Create PostgreSQL 16 instance
2. Configure security group (allow connections from bot)
3. Enable automated backups
4. Copy connection string to `DATABASE_URL`

**Google Cloud SQL**:
1. Create PostgreSQL instance
2. Enable Cloud SQL Proxy or allow bot IP
3. Copy connection string

**DigitalOcean Managed Database**:
1. Create PostgreSQL cluster
2. Add bot as trusted source
3. Copy connection string (use connection pooling URL)

### Run Migrations

```bash
# From deployment server or CI/CD
export DATABASE_URL="postgres://user:pass@db-host:5432/incident_bot"
sqlx migrate run

# Or from Docker
docker run --rm \
  -v $(pwd)/migrations:/migrations \
  -e DATABASE_URL="postgres://..." \
  incident-bot:latest \
  sqlx migrate run
```

---

## Reverse Proxy / Load Balancer

### Nginx

```nginx
upstream incident_bot {
    server localhost:3000;
}

server {
    listen 443 ssl http2;
    server_name incidents.yourcompany.com;

    ssl_certificate /etc/letsencrypt/live/incidents.yourcompany.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/incidents.yourcompany.com/privkey.pem;

    location / {
        proxy_pass http://incident_bot;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /health {
        proxy_pass http://incident_bot/health;
        access_log off;
    }
}
```

### Caddy

```caddyfile
incidents.yourcompany.com {
    reverse_proxy localhost:3000
}
```

---

## Monitoring & Observability

### Health Checks

```bash
# Basic health
curl https://incidents.yourcompany.com/health
# Expected: "OK"

# Database connectivity
# Health endpoint already checks DB connection
```

### Logs

**Docker**:
```bash
docker logs -f incident-bot --tail 100
```

**Kubernetes**:
```bash
kubectl -n incident-bot logs -f deployment/incident-bot
```

**Systemd**:
```bash
sudo journalctl -u incident-bot -f
```

### Log Aggregation

Logs are output as **JSON** for easy parsing:

```json
{"timestamp":"2025-02-15T12:34:56Z","level":"INFO","target":"incident_bot","message":"Incident 123 declared"}
```

**Recommended tools**:
- ELK Stack (Elasticsearch, Logstash, Kibana)
- Datadog
- CloudWatch Logs (AWS)
- Cloud Logging (Google Cloud)
- Loki + Grafana

### Metrics (Future Enhancement)

Planned `/metrics` endpoint for Prometheus scraping. For now, monitor:
- HTTP response times (via reverse proxy)
- Database connection pool (via PostgreSQL monitoring)
- Error rates (via log aggregation)

---

## Security Hardening

### Secrets Management

**Kubernetes Secrets**:
```bash
kubectl create secret generic incident-bot-secrets \
  --from-literal=slack-bot-token="xoxb-..." \
  --from-literal=slack-signing-secret="..." \
  -n incident-bot
```

**AWS Secrets Manager**:
```bash
aws secretsmanager create-secret \
  --name incident-bot/slack-bot-token \
  --secret-string "xoxb-..."
```

**HashiCorp Vault**:
```bash
vault kv put secret/incident-bot \
  slack_bot_token="xoxb-..." \
  slack_signing_secret="..."
```

### Network Security

- **Firewall**: Only allow HTTPS (443) and health checks
- **IP Allowlisting**: Restrict to Slack's IP ranges (if supported)
- **TLS**: Enforce HTTPS with modern cipher suites
- **Rate Limiting**: Implement at reverse proxy or API gateway

### Application Security

- ✅ Slack signature verification enabled (HMAC-SHA256)
- ✅ SQL injection prevention (sqlx compile-time queries)
- ✅ Commander-only permissions enforced
- ✅ No secrets in logs

---

## Backup & Disaster Recovery

### Database Backups

**Automated backups** (managed PostgreSQL):
- Enable daily automated backups
- Retain for 7-30 days
- Test restore procedure quarterly

**Manual backup**:
```bash
pg_dump -h db-host -U user -d incident_bot > backup_$(date +%Y%m%d).sql
```

**Restore**:
```bash
psql -h db-host -U user -d incident_bot < backup_20250215.sql
```

### Application State

Bot is **stateless** - no local data to back up. All state in PostgreSQL.

### Disaster Recovery Checklist

- [ ] Database backups tested and verified
- [ ] Slack app credentials documented
- [ ] Deployment procedure documented
- [ ] DNS records documented
- [ ] Monitoring/alerting configured

---

## Scaling

### Horizontal Scaling

Bot is **stateless** and can scale horizontally:

**Docker**:
```bash
docker compose up -d --scale app=3
```

**Kubernetes**:
```bash
kubectl -n incident-bot scale deployment incident-bot --replicas=3
```

**Considerations**:
- Slack guarantees at-least-once delivery (duplicate requests possible)
- Database connection pool shared across instances
- No coordination needed between instances

### Vertical Scaling

Resource recommendations:

| Load | CPU | Memory | DB Connections |
|------|-----|--------|----------------|
| Light (<10 incidents/day) | 0.25 vCPU | 256 MB | 5 |
| Medium (<50 incidents/day) | 0.5 vCPU | 512 MB | 10 |
| Heavy (>50 incidents/day) | 1 vCPU | 1 GB | 20 |

---

## Rollback Procedure

### Docker/Kubernetes

```bash
# Rollback to previous version
kubectl -n incident-bot rollout undo deployment/incident-bot

# Or specific revision
kubectl -n incident-bot rollout undo deployment/incident-bot --to-revision=2
```

### Binary Deployment

```bash
# Keep previous binary
sudo systemctl stop incident-bot
sudo mv /opt/incident-bot/incident-bot /opt/incident-bot/incident-bot.new
sudo mv /opt/incident-bot/incident-bot.old /opt/incident-bot/incident-bot
sudo systemctl start incident-bot
```

### Database Migrations

**Migrations are forward-only**. To rollback:
1. Deploy previous application version
2. Manually revert schema changes if needed (risky)
3. Better: Design migrations to be backward-compatible

---

## Post-Deployment Verification

```bash
# 1. Health check
curl https://incidents.yourcompany.com/health
# Expected: "OK"

# 2. Test slash command in Slack
/incident declare

# 3. Check logs for errors
# (See Logs section above)

# 4. Verify database connection
psql $DATABASE_URL -c "SELECT COUNT(*) FROM incidents;"

# 5. Test incident lifecycle
# - Declare incident
# - Post status update
# - Change severity
# - Resolve
```

---

## Troubleshooting Production Issues

### Bot not responding to Slack commands

**Check**:
1. Service is running: `curl https://your-domain/health`
2. Logs show incoming requests
3. Slack signing secret matches
4. Request URL in Slack app config is correct

### Database connection errors

**Check**:
1. PostgreSQL is running
2. Network connectivity from bot to DB
3. Credentials are correct
4. Connection pool not exhausted

### Statuspage not updating

**Check**:
1. API key and page ID configured
2. Component mappings exist in database
3. Logs show job worker is running
4. Network connectivity to Statuspage API

---

## Support

For deployment issues:
1. Check logs with `RUST_LOG=incident_bot=debug`
2. Review [CONFIGURATION.md](./CONFIGURATION.md) for config issues
3. Review [SLACK_SETUP.md](./SLACK_SETUP.md) for Slack integration
4. Create GitHub issue with logs and config (redact secrets)
