use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use std::fmt::Display;

#[derive(Serialize)]
struct ErrorResponse {
    error: Option<String>,
    details: Option<String>,
}

pub struct AppError {
    pub code: StatusCode,
    pub message: Option<String>,
    pub details: Option<String>,
}

impl AppError {
    pub fn with_status(code: StatusCode) -> Self {
        Self {
            code,
            message: None,
            details: None,
        }
    }

    pub fn with_message<S: Into<String>>(code: StatusCode, message: S) -> Self {
        Self {
            code,
            message: Some(message.into()),
            details: None,
        }
    }

    pub fn with_details<E: std::fmt::Debug>(
        code: StatusCode,
        message: Option<String>,
        err: Option<E>,
    ) -> Self {
        Self {
            code,
            message,
            details: err.map(|e| format!("{:#?}", e)),
        }
    }
}

impl<E> From<E> for AppError
where
    E: std::fmt::Display + std::error::Error,
{
    fn from(value: E) -> Self {
        AppError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: Some("Internal Server Error".to_string()),
            details: Some(value.to_string()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(ErrorResponse {
            error: self.message,
            details: self.details,
        });
        (self.code, body).into_response()
    }
}
