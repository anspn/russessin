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

/// GET /api/v1/users
pub async fn list_users(State(client): State<Client>) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Listing all users");
    let users = client.list_users().await?;
    Ok(Json(users))
}

/// GET /api/v1/users/:uid
pub async fn show_user(
    State(client): State<Client>,
    Path(uid): Path<u32>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(uid = uid, "Showing user");
    let props = client.show_user(uid).await?;
    Ok(Json(props))
}

/// GET /api/v1/users/:uid/status
pub async fn user_status(
    State(client): State<Client>,
    Path(uid): Path<u32>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(uid = uid, "Getting user status");
    let status = client.user_status(uid).await?;
    Ok(Json(status))
}

/// POST /api/v1/users/:uid/linger
pub async fn enable_linger(
    State(client): State<Client>,
    Path(uid): Path<u32>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(uid = uid, "Enabling linger");
    client.enable_linger(uid).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/users/:uid/linger
pub async fn disable_linger(
    State(client): State<Client>,
    Path(uid): Path<u32>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(uid = uid, "Disabling linger");
    client.disable_linger(uid).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/users/:uid
pub async fn terminate_user(
    State(client): State<Client>,
    Path(uid): Path<u32>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(uid = uid, "Terminating user");
    client.terminate_user(uid).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/users/:uid/kill
pub async fn kill_user(
    State(client): State<Client>,
    Path(uid): Path<u32>,
    Json(body): Json<KillRequest>,
) -> Result<impl IntoResponse, AppError> {
    let signal = parse_signal(&body.signal)?;
    tracing::info!(uid = uid, signal = %body.signal, "Killing user");
    client.kill_user(uid, signal).await?;
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
            .route("/users", get(list_users))
            .route("/users/{uid}", get(show_user).delete(terminate_user))
            .route("/users/{uid}/status", get(user_status))
            .route("/users/{uid}/linger", post(enable_linger).delete(disable_linger))
            .route("/users/{uid}/kill", post(kill_user))
            .with_state(client)
    }

    async fn response_body(resp: axum::http::Response<Body>) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn sample_user_info() -> UserInfo {
        UserInfo {
            uid: 1000,
            name: "testuser".to_string(),
            path: "/org/freedesktop/login1/user/_31000".to_string(),
        }
    }

    fn sample_user_properties() -> UserProperties {
        UserProperties {
            uid: 1000,
            name: "testuser".to_string(),
            state: "active".to_string(),
            linger: false,
            runtime_path: "/run/user/1000".to_string(),
            service: "user@1000.service".to_string(),
            slice: "user-1000.slice".to_string(),
            display: "42".to_string(),
            timestamp: 1700000000,
            sessions: vec!["42".to_string()],
        }
    }

    #[tokio::test]
    async fn test_list_users() {
        let mut mock = MockLogindClient::new();
        mock.expect_list_users()
            .returning(|| Ok(vec![sample_user_info()]));

        let app = test_router(mock);
        let resp = app
            .oneshot(Request::get("/users").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_body(resp).await;
        let users: Vec<UserInfo> = serde_json::from_str(&body).unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].uid, 1000);
    }

    #[tokio::test]
    async fn test_show_user() {
        let mut mock = MockLogindClient::new();
        mock.expect_show_user()
            .withf(|uid| *uid == 1000)
            .returning(|_| Ok(sample_user_properties()));

        let app = test_router(mock);
        let resp = app
            .oneshot(Request::get("/users/1000").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_body(resp).await;
        let props: UserProperties = serde_json::from_str(&body).unwrap();
        assert_eq!(props.uid, 1000);
        assert_eq!(props.name, "testuser");
    }

    #[tokio::test]
    async fn test_user_status() {
        let mut mock = MockLogindClient::new();
        mock.expect_user_status()
            .withf(|uid| *uid == 1000)
            .returning(|_| {
                let props = sample_user_properties();
                Ok(UserStatus {
                    uid: 1000,
                    name: "testuser".to_string(),
                    properties: props,
                })
            });

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::get("/users/1000/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_body(resp).await;
        let status: UserStatus = serde_json::from_str(&body).unwrap();
        assert_eq!(status.uid, 1000);
    }

    #[tokio::test]
    async fn test_enable_linger() {
        let mut mock = MockLogindClient::new();
        mock.expect_enable_linger()
            .withf(|uid| *uid == 1000)
            .returning(|_| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/users/1000/linger")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_disable_linger() {
        let mut mock = MockLogindClient::new();
        mock.expect_disable_linger()
            .withf(|uid| *uid == 1000)
            .returning(|_| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::delete("/users/1000/linger")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_terminate_user() {
        let mut mock = MockLogindClient::new();
        mock.expect_terminate_user()
            .withf(|uid| *uid == 1000)
            .returning(|_| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::delete("/users/1000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_kill_user() {
        let mut mock = MockLogindClient::new();
        mock.expect_kill_user()
            .withf(|uid, signal| *uid == 1000 && *signal == 9)
            .returning(|_, _| Ok(()));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/users/1000/kill")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"signal": "SIGKILL"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_list_users_dbus_error() {
        let mut mock = MockLogindClient::new();
        mock.expect_list_users()
            .returning(|| Err(AppError::Dbus("connection refused".to_string())));

        let app = test_router(mock);
        let resp = app
            .oneshot(Request::get("/users").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_show_user_not_found() {
        let mut mock = MockLogindClient::new();
        mock.expect_show_user()
            .returning(|uid| Err(AppError::NotFound(format!("user {uid} not found"))));

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::get("/users/99999").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_kill_user_invalid_signal() {
        let mock = MockLogindClient::new();

        let app = test_router(mock);
        let resp = app
            .oneshot(
                Request::post("/users/1000/kill")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"signal": "INVALID"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
