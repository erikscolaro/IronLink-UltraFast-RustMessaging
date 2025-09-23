use axum::response::Json;
use serde_json::json;

pub async fn root() -> Json<serde_json::Value> {
	Json(json!({"message": "Hello, Axum!"}))
}

