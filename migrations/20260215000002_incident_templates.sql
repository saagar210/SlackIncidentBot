-- Create incident templates table for pre-defined common incident scenarios
CREATE TABLE IF NOT EXISTS incident_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL CHECK (length(title) <= 100),
    severity TEXT NOT NULL CHECK (severity IN ('P1', 'P2', 'P3', 'P4')),
    affected_service TEXT,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for active templates lookup
CREATE INDEX idx_templates_active ON incident_templates(is_active) WHERE is_active = true;

-- Seed default templates for common scenarios
INSERT INTO incident_templates (name, title, severity, affected_service, description) VALUES
    ('database-outage', 'Database Outage', 'P1', 'database', 'Complete database unavailability affecting all services'),
    ('api-degradation', 'API Performance Degradation', 'P2', 'api-gateway', 'API response times elevated, degraded user experience'),
    ('payment-failure', 'Payment Processing Failure', 'P1', 'payment-processor', 'Payments failing to process, revenue impact'),
    ('auth-slowness', 'Authentication Service Slowness', 'P2', 'auth-service', 'Login/authentication experiencing delays'),
    ('cdn-issues', 'CDN Performance Issues', 'P3', 'cdn', 'Static assets loading slowly or intermittently'),
    ('security-breach', 'Security Incident', 'P1', NULL, 'Potential security breach detected, immediate investigation required'),
    ('deployment-rollback', 'Failed Deployment Requiring Rollback', 'P2', NULL, 'Recent deployment causing issues, rollback needed'),
    ('third-party-outage', 'Third-Party Service Outage', 'P3', NULL, 'External dependency is down, monitoring impact');
