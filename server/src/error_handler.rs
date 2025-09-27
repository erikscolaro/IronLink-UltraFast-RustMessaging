use axum::{http::StatusCode, response::IntoResponse, Json};
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
    pub fn new<E: std::fmt::Debug>(
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

    pub fn from_status(code: StatusCode) -> Self {
        Self {
            code,
            message: None,
            details: None,
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
