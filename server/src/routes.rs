use crate::handlers;
use axum::Router;

pub fn router() -> Router {
    Router::new().route("/", axum::routing::get(handlers::root))
}
