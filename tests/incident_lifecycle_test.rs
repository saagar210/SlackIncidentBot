use incident_bot::db::models::{Severity, TimelineEventType};
use incident_bot::services::incident::IncidentService;
use incident_bot::services::timeline::TimelineService;

mod common;

#[tokio::test]
async fn test_create_incident() {
    let ctx = common::TestContext::new().await;

    let incident_service = IncidentService::new(ctx.pool.clone());

    // Create incident
    let incident = incident_service
        .create_incident(
            "Test incident".to_string(),
            Severity::P2,
            "Test Service".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to create incident");

    assert_eq!(incident.title, "Test incident");
    assert_eq!(incident.severity, Severity::P2);
    assert_eq!(incident.affected_service, "Test Service");
    assert_eq!(incident.commander_id, "U024COMMANDER");

    // Verify timeline was created
    let timeline_service = TimelineService::new(ctx.pool.clone());
    let timeline = timeline_service
        .get_timeline(incident.id)
        .await
        .expect("Failed to get timeline");

    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].event_type, TimelineEventType::Declared);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_post_status_update() {
    let ctx = common::TestContext::new().await;

    let incident_service = IncidentService::new(ctx.pool.clone());

    // Create incident
    let incident = incident_service
        .create_incident(
            "Test incident".to_string(),
            Severity::P2,
            "Test Service".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to create incident");

    // Post status update as commander
    let result = incident_service
        .post_status_update(
            incident.id,
            "Investigating issue".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await;

    assert!(result.is_ok());

    // Verify timeline has 2 events
    let timeline_service = TimelineService::new(ctx.pool.clone());
    let timeline = timeline_service
        .get_timeline(incident.id)
        .await
        .expect("Failed to get timeline");

    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[1].event_type, TimelineEventType::StatusUpdate);
    assert_eq!(timeline[1].message, "Investigating issue");

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_non_commander_cannot_update() {
    let ctx = common::TestContext::new().await;

    let incident_service = IncidentService::new(ctx.pool.clone());

    // Create incident with commander U024COMMANDER
    let incident = incident_service
        .create_incident(
            "Test incident".to_string(),
            Severity::P2,
            "Test Service".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to create incident");

    // Try to post status update as different user
    let result = incident_service
        .post_status_update(
            incident.id,
            "Unauthorized update".to_string(),
            "U024OTHER".to_string(),
        )
        .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        incident_bot::error::IncidentError::PermissionDenied { .. }
    ));

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_change_severity() {
    let ctx = common::TestContext::new().await;

    let incident_service = IncidentService::new(ctx.pool.clone());

    // Create P2 incident
    let incident = incident_service
        .create_incident(
            "Test incident".to_string(),
            Severity::P2,
            "Test Service".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to create incident");

    // Change to P1
    let (updated, old_severity) = incident_service
        .change_severity(
            incident.id,
            Severity::P1,
            "U024COMMANDER".to_string(),
            Some("Impact increased".to_string()),
        )
        .await
        .expect("Failed to change severity");

    assert_eq!(old_severity, Severity::P2);
    assert_eq!(updated.severity, Severity::P1);

    // Verify timeline
    let timeline_service = TimelineService::new(ctx.pool.clone());
    let timeline = timeline_service
        .get_timeline(incident.id)
        .await
        .expect("Failed to get timeline");

    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[1].event_type, TimelineEventType::SeverityChange);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_resolve_incident() {
    let ctx = common::TestContext::new().await;

    let incident_service = IncidentService::new(ctx.pool.clone());

    // Create incident
    let incident = incident_service
        .create_incident(
            "Test incident".to_string(),
            Severity::P2,
            "Test Service".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to create incident");

    // Wait a bit for duration calculation
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Resolve
    let resolved = incident_service
        .resolve_incident(incident.id, "U024COMMANDER".to_string())
        .await
        .expect("Failed to resolve incident");

    assert!(resolved.status.is_terminal());
    assert!(resolved.resolved_at.is_some());
    assert!(resolved.duration_minutes.is_some());
    assert!(resolved.duration_minutes.unwrap() >= 0);

    // Verify timeline
    let timeline_service = TimelineService::new(ctx.pool.clone());
    let timeline = timeline_service
        .get_timeline(incident.id)
        .await
        .expect("Failed to get timeline");

    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[1].event_type, TimelineEventType::Resolved);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_resolve_idempotent() {
    let ctx = common::TestContext::new().await;

    let incident_service = IncidentService::new(ctx.pool.clone());

    // Create and resolve incident
    let incident = incident_service
        .create_incident(
            "Test incident".to_string(),
            Severity::P2,
            "Test Service".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to create incident");

    incident_service
        .resolve_incident(incident.id, "U024COMMANDER".to_string())
        .await
        .expect("Failed to resolve incident");

    // Resolve again (should succeed idempotently)
    let result = incident_service
        .resolve_incident(incident.id, "U024COMMANDER".to_string())
        .await;

    assert!(result.is_ok());

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_full_incident_lifecycle() {
    let ctx = common::TestContext::new().await;

    let incident_service = IncidentService::new(ctx.pool.clone());
    let timeline_service = TimelineService::new(ctx.pool.clone());

    // 1. Declare incident
    let incident = incident_service
        .create_incident(
            "Full lifecycle test".to_string(),
            Severity::P3,
            "Test Service".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to create incident");

    // 2. Post status update
    incident_service
        .post_status_update(
            incident.id,
            "Investigating".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to post status");

    // 3. Escalate to P1
    incident_service
        .change_severity(
            incident.id,
            Severity::P1,
            "U024COMMANDER".to_string(),
            Some("Impact increased".to_string()),
        )
        .await
        .expect("Failed to change severity");

    // 4. Another status update
    incident_service
        .post_status_update(
            incident.id,
            "Fix deployed".to_string(),
            "U024COMMANDER".to_string(),
        )
        .await
        .expect("Failed to post status");

    // 5. Resolve
    let resolved = incident_service
        .resolve_incident(incident.id, "U024COMMANDER".to_string())
        .await
        .expect("Failed to resolve incident");

    // Verify timeline has all events
    let timeline = timeline_service
        .get_timeline(incident.id)
        .await
        .expect("Failed to get timeline");

    assert_eq!(timeline.len(), 5);
    assert_eq!(timeline[0].event_type, TimelineEventType::Declared);
    assert_eq!(timeline[1].event_type, TimelineEventType::StatusUpdate);
    assert_eq!(timeline[2].event_type, TimelineEventType::SeverityChange);
    assert_eq!(timeline[3].event_type, TimelineEventType::StatusUpdate);
    assert_eq!(timeline[4].event_type, TimelineEventType::Resolved);

    // Verify final state
    assert_eq!(resolved.severity, Severity::P1);
    assert!(resolved.status.is_terminal());
    assert!(resolved.duration_minutes.is_some());

    ctx.cleanup().await;
}
