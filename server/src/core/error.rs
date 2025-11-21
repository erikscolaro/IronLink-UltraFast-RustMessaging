use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

pub struct AppError {
    status: StatusCode,
    message: &'static str,
    details: Option<String>,
}

impl AppError {
    pub fn new(status: StatusCode, message: &'static str) -> Self {
        Self {
            status,
            message,
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    // Common error constructors
    pub fn not_found(message: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request(message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: &'static str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: &'static str) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn conflict(message: &'static str) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    pub fn internal_server_error(message: &'static str) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn service_unavailable(message: &'static str) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, message)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::not_found("Resource not found"),

            sqlx::Error::Database(_) => Self::bad_request("Database error"),

            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => {
                Self::service_unavailable("Database unavailable")
            }

            _ => Self::internal_server_error("Internal server error"),
        }
    }
}

impl From<axum::Error> for AppError {
    fn from(err: axum::Error) -> Self {
        Self::internal_server_error("Internal server error").with_details(err.to_string())
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        Self::bad_request("Validation error").with_details(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(ErrorResponse {
            error: self.message,
            details: self.details,
        });
        (self.status, body).into_response()
    }
}
