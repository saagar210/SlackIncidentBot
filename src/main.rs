use axum::routing::{get, post};
use axum::Router;
use incident_bot::adapters::statuspage::StatuspageClient;
use incident_bot::jobs::worker::JobWorker;
use incident_bot::{db, AppConfig, AppState};
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "incident_bot=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("Starting Incident Bot");

    // Load configuration
    let config = AppConfig::from_env().expect("Failed to load configuration");
    config.validate().expect("Configuration validation failed");

    info!("Configuration loaded");

    // Create database pool
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    // Create Statuspage client (if configured)
    let statuspage_client = if let (Some(api_key), Some(page_id)) =
        (&config.statuspage_api_key, &config.statuspage_page_id)
    {
        info!("Statuspage integration enabled");
        Some(StatuspageClient::new(api_key.clone(), page_id.clone()))
    } else {
        info!("Statuspage integration disabled (no API key configured)");
        None
    };

    // Create job queue
    let (job_sender, job_receiver) = mpsc::unbounded_channel();

    // Start job worker
    let worker = JobWorker::new(job_receiver, statuspage_client);
    tokio::spawn(async move {
        worker.start().await;
    });

    // Create app state
    let state = AppState::new(pool.clone(), config.clone(), job_sender);

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/slack/commands", post(incident_bot::slack::events::handle_slash_command))
        .route("/slack/interactions", post(incident_bot::slack::events::handle_interaction))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;
    use axum::Json;

    let db_healthy = db::health_check(&state.pool).await;

    if db_healthy {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "healthy",
                "database": "connected",
                "version": env!("CARGO_PKG_VERSION"),
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "unhealthy",
                "database": "disconnected",
                "version": env!("CARGO_PKG_VERSION"),
            })),
        )
    }
}
