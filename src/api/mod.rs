pub mod sessions;
pub mod users;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;

use crate::logind::LogindClient;

pub fn router(client: Arc<dyn LogindClient>) -> Router {
    let session_routes = Router::new()
        .route("/", get(sessions::list_sessions))
        .route("/lock-all", post(sessions::lock_all_sessions))
        .route("/unlock-all", post(sessions::unlock_all_sessions))
        .route(
            "/{id}",
            get(sessions::show_session).delete(sessions::terminate_session),
        )
        .route("/{id}/status", get(sessions::session_status))
        .route("/{id}/activate", post(sessions::activate_session))
        .route("/{id}/lock", post(sessions::lock_session))
        .route("/{id}/unlock", post(sessions::unlock_session))
        .route("/{id}/kill", post(sessions::kill_session));

    let user_routes = Router::new()
        .route("/", get(users::list_users))
        .route(
            "/{uid}",
            get(users::show_user).delete(users::terminate_user),
        )
        .route("/{uid}/status", get(users::user_status))
        .route(
            "/{uid}/linger",
            post(users::enable_linger).delete(users::disable_linger),
        )
        .route("/{uid}/kill", post(users::kill_user));

    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api/v1/sessions", session_routes)
        .nest("/api/v1/users", user_routes)
        .with_state(client)
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}
