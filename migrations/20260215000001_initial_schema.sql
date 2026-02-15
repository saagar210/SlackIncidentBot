-- Incidents table: core incident metadata
CREATE TABLE incidents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slack_channel_id TEXT UNIQUE,
    title TEXT NOT NULL CHECK (length(title) <= 100),
    severity TEXT NOT NULL CHECK (severity IN ('P1', 'P2', 'P3', 'P4')),
    status TEXT NOT NULL DEFAULT 'declared'
        CHECK (status IN ('declared', 'investigating', 'identified', 'monitoring', 'resolved')),
    affected_service TEXT NOT NULL,
    commander_id TEXT NOT NULL,
    declared_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    duration_minutes INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_incidents_service ON incidents(affected_service);
CREATE INDEX idx_incidents_severity ON incidents(severity);
CREATE INDEX idx_incidents_resolved_at ON incidents(resolved_at);
CREATE INDEX idx_incidents_commander ON incidents(commander_id);
CREATE INDEX idx_incidents_channel ON incidents(slack_channel_id);
CREATE INDEX idx_incidents_channel_status ON incidents(slack_channel_id, status);

-- Timeline table: immutable event log
CREATE TABLE incident_timeline (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id UUID NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL
        CHECK (event_type IN ('declared', 'status_update', 'severity_change', 'resolved')),
    message TEXT NOT NULL,
    posted_by TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_timeline_incident ON incident_timeline(incident_id, timestamp);

-- Notifications table: audit trail of who was notified when
CREATE TABLE incident_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id UUID NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    notification_type TEXT NOT NULL
        CHECK (notification_type IN ('slack_channel', 'slack_dm')),
    recipient TEXT NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL DEFAULT 'sent'
        CHECK (status IN ('sent', 'failed', 'pending')),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_incident ON incident_notifications(incident_id);
CREATE INDEX idx_notifications_recipient ON incident_notifications(recipient, sent_at);

-- Statuspage component mappings: service → Statuspage component ID
CREATE TABLE statuspage_mappings (
    service_name TEXT PRIMARY KEY,
    component_id TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Audit log: every command + state change
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id UUID REFERENCES incidents(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    actor_email TEXT,
    old_state JSONB,
    new_state JSONB,
    details JSONB,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_incident ON audit_log(incident_id);
CREATE INDEX idx_audit_actor ON audit_log(actor_id, timestamp);
CREATE INDEX idx_audit_action ON audit_log(action, timestamp);
