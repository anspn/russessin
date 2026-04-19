pub mod client;
#[cfg(test)]
pub mod mock;
pub mod types;

use async_trait::async_trait;
use crate::error::AppError;
use types::*;

/// Trait abstracting interactions with systemd-logind.
/// Implementations can use D-Bus (production) or be mocked (testing).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LogindClient: Send + Sync + 'static {
    // Session commands
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, AppError>;
    async fn session_status(&self, id: &str) -> Result<SessionStatus, AppError>;
    async fn show_session(&self, id: &str) -> Result<SessionProperties, AppError>;
    async fn activate_session(&self, id: &str) -> Result<(), AppError>;
    async fn lock_session(&self, id: &str) -> Result<(), AppError>;
    async fn unlock_session(&self, id: &str) -> Result<(), AppError>;
    async fn lock_sessions(&self) -> Result<(), AppError>;
    async fn unlock_sessions(&self) -> Result<(), AppError>;
    async fn terminate_session(&self, id: &str) -> Result<(), AppError>;
    async fn kill_session(&self, id: &str, who: &str, signal: i32) -> Result<(), AppError>;

    // User commands
    async fn list_users(&self) -> Result<Vec<UserInfo>, AppError>;
    async fn user_status(&self, uid: u32) -> Result<UserStatus, AppError>;
    async fn show_user(&self, uid: u32) -> Result<UserProperties, AppError>;
    async fn enable_linger(&self, uid: u32) -> Result<(), AppError>;
    async fn disable_linger(&self, uid: u32) -> Result<(), AppError>;
    async fn terminate_user(&self, uid: u32) -> Result<(), AppError>;
    async fn kill_user(&self, uid: u32, signal: i32) -> Result<(), AppError>;
}
