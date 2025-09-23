use axum::Router;
use crate::handlers;

pub fn router() -> Router {
	Router::new()
		.route("/", axum::routing::get(handlers::root))
}
