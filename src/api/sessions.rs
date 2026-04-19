use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::Arc;

use crate::error::AppError;
use crate::logind::types::KillRequest;
use crate::logind::LogindClient;

type Client = Arc<dyn LogindClient>;

fn parse_signal(name: &str) -> Result<i32, AppError> {
    match name.to_uppercase().as_str() {
        "SIGHUP" | "HUP" | "1" => Ok(1),
        "SIGINT" | "INT" | "2" => Ok(2),
        "SIGQUIT" | "QUIT" | "3" => Ok(3),
        "SIGKILL" | "KILL" | "9" => Ok(9),
        "SIGTERM" | "TERM" | "15" => Ok(15),
        "SIGUSR1" | "USR1" | "10" => Ok(10),
        "SIGUSR2" | "USR2" | "12" => Ok(12),
        other => other
            .parse::<i32>()
            .map_err(|_| AppError::BadRequest(format!("invalid signal: {other}"))),
    }
}

/// GET /api/v1/sessions
pub async fn list_sessions(State(client): State<Client>) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Listing all sessions");
    let sessions = client.list_sessions().await?;
    Ok(Json(sessions))
}

/// GET /api/v1/sessions/:id
pub async fn show_session(
    State(client): State<Client>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(session_id = %id, "Showing session");
    let props = client.show_session(&id).await?;
    Ok(Json(props))
}

/// GET /api/v1/sessions/:id/status
pub async fn session_status(
    State(client): State<Client>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(session_id = %id, "Getting session status");
    let status = client.session_status(&id).await?;
    Ok(Json(status))
}

/// POST /api/v1/sessions/:id/activate
pub async fn activate_session(
    State(client): State<Client>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(session_id = %id, "Activating session");
    client.activate_session(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/sessions/:id/lock
pub async fn lock_session(
    State(client): State<Client>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(session_id = %id, "Locking session");
    client.lock_session(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/sessions/:id/unlock
pub async fn unlock_session(
    State(client): State<Client>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(session_id = %id, "Unlocking session");
    client.unlock_session(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/sessions/lock-all
pub async fn lock_all_sessions(
    State(client): State<Client>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Locking all sessions");
    client.lock_sessions().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/sessions/unlock-all
pub async fn unlock_all_sessions(
    State(client): State<Client>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Unlocking all sessions");
    client.unlock_sessions().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/sessions/:id
pub async fn terminate_session(
    State(client): State<Client>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(session_id = %id, "Terminating session");
    client.terminate_session(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/sessions/:id/kill
pub async fn kill_session(
    State(client): State<Client>,
    Path(id): Path<String>,
    Json(body): Json<KillRequest>,
) -> Result<impl IntoResponse, AppError> {
    let signal = parse_signal(&body.signal)?;
    let who = body.who.as_deref().unwrap_or("all");
    tracing::info!(session_id = %id, signal = %body.signal, who = %who, "Killing session");
    client.kill_session(&id, who, signal).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logind::types::*;
    use crate::logind::MockLogindClient;
    use axum::body::Body;
    use axum::http::Request;
    use axum::Router;
    use axum::routing::{delete, get, post};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_router(mock: MockLogindClient) -> Router {
        let client: Client = Arc::new(mock);
        Router::new()
            .route("/sessions", get(list_sessions))
            .route("/sessions/lock-all", post(lock_all_sessions))
            .route("/sessions/unlock-all", post(unlock_all_sessions))
            .route("/sessions/{id}", get(show_session).delete(terminate_session))
            .route("/sessions/{id}/status", get(session_status))
            .route("/sessions/{id}/activate", post(activate_session))
            .route("/sessions/{id}/lock", post(lock_session))
            .route("/sessions/{id}/unlock", post(unlock_session))
            .route("/sessions/{id}/kill", post(kill_session))
            .with_state(client)
    }

    async fn response_body(resp: axum::http::Response<Body>) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn sample_session_info() -> SessionInfo {
        SessionInfo {
            id: "42".to_string(),
            uid: 1000,
            user: "testuser".to_string(),
            seat: "seat0".to_string(),
            path: "/org/freedesktop/login1/session/_342".to_string(),
        }
    }

    fn sample_session_properties() -> SessionProperties {
        SessionProperties {
            id: "42".to_string(),
            uid: 1000,
            user: "testuser".to_string(),
            seat: "seat0".to_string(),
            session_type: "x11".to_string(),
            class: "user".to_string(),
            active: true,
            state: "active".to_string(),
            remote: false,
            remote_host: String::new(),
            remote_user: String::new(),
            service: "gdm-password".to_string(),
            desktop: "gnome".to_string(),
            scope: "session-42.scope".to_string(),
            leader: 1234,
            audit: 42,
            vt_nr: 2,
            tty: String::new(),
            display: ":1".to_string(),
            timestamp: 1700000000,
        }
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let mut mock = MockLogindClient::new();
        mock.expect_list_sessions()
            .returning(|| Ok(vec![sample_session_info()]));

        let app = test_router(mock);
        let resp = app
            .oneshot(Request::get("/sessions").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_body(resp).await;
        let sessions: Vec<SessionInfo> = serde_json::from_str(&body).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "42");
    }

    #[tokio::test]
    async fn test_show_session() {
        let mut mock = MockLogindClient::new();
        mock.expect_show_session()
            .withf(|id| id == "42")
            .returning(|_| Ok(sample_session_properties()));

        let app = test_router(mock);
        let resp = app
            .oneshot(Request::get("/sessions/42").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_body(resp).await;
        let props: SessionProperties = serde_json::from_str(&body).unwrap();
        assert_eq!(props.id, "42");
        assert!(props.active);
    }

    #[tokio::test]
    async fn test_session_status() {
        let mut mock = MockLogindClient::new();
        mock.expect_session_status()
            .withf(|id| id == "42")
            .returning(|_| {
                Ok(SessionStatus {
                    id: "42".to_string(),
                    properties: sample_session_properties(),
                })
            });

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::get("/sessions/42/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_body(resp).await;
        let status: SessionStatus = serde_json::from_str(&body).unwrap();
        assert_eq!(status.id, "42");
    }

    #[tokio::test]
    async fn test_activate_session() {
        let mut mock = MockLogindClient::new();
        mock.expect_activate_session()
            .withf(|id| id == "42")
            .returning(|_| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/sessions/42/activate")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_lock_session() {
        let mut mock = MockLogindClient::new();
        mock.expect_lock_session()
            .withf(|id| id == "42")
            .returning(|_| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/sessions/42/lock")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_unlock_session() {
        let mut mock = MockLogindClient::new();
        mock.expect_unlock_session()
            .withf(|id| id == "42")
            .returning(|_| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/sessions/42/unlock")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_lock_all_sessions() {
        let mut mock = MockLogindClient::new();
        mock.expect_lock_sessions().returning(|| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/sessions/lock-all")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_unlock_all_sessions() {
        let mut mock = MockLogindClient::new();
        mock.expect_unlock_sessions().returning(|| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/sessions/unlock-all")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_terminate_session() {
        let mut mock = MockLogindClient::new();
        mock.expect_terminate_session()
            .withf(|id| id == "42")
            .returning(|_| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::delete("/sessions/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_kill_session() {
        let mut mock = MockLogindClient::new();
        mock.expect_kill_session()
            .withf(|id, who, signal| id == "42" && who == "all" && *signal == 15)
            .returning(|_, _, _| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/sessions/42/kill")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"signal": "SIGTERM"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_list_sessions_dbus_error() {
        let mut mock = MockLogindClient::new();
        mock.expect_list_sessions()
            .returning(|| Err(AppError::Dbus("connection refused".to_string())));

        let app = test_router(mock);
        let resp = app
            .oneshot(Request::get("/sessions").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_show_session_not_found() {
        let mut mock = MockLogindClient::new();
        mock.expect_show_session()
            .returning(|id| Err(AppError::NotFound(format!("session '{id}' not found"))));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::get("/sessions/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
