use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IncidentError {
    #[error("Incident not found")]
    NotFound,

    #[error("Permission denied: {user_id} cannot {action}")]
    PermissionDenied { user_id: String, action: String },

    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: crate::db::models::IncidentStatus,
        to: crate::db::models::IncidentStatus,
    },

    #[error("Slack API error: {message} (code: {slack_error_code})")]
    SlackAPIError {
        message: String,
        slack_error_code: String,
    },

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("External API error ({service}): {message}")]
    ExternalAPIError { service: String, message: String },

    #[error("Validation error on field '{field}': {reason}")]
    ValidationError { field: String, reason: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid Slack signature")]
    InvalidSignature,

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type IncidentResult<T> = Result<T, IncidentError>;

impl IntoResponse for IncidentError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            IncidentError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            IncidentError::PermissionDenied { .. } => (StatusCode::FORBIDDEN, self.to_string()),
            IncidentError::ValidationError { .. } => (StatusCode::BAD_REQUEST, self.to_string()),
            IncidentError::InvalidSignature => (StatusCode::UNAUTHORIZED, self.to_string()),
            IncidentError::InvalidStateTransition { .. } => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
