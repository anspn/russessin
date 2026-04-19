use serde::{Deserialize, Serialize};

/// Summary information about a login session (from ListSessions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub uid: u32,
    pub user: String,
    pub seat: String,
    pub path: String,
}

/// Detailed session properties (from GetSession / show-session)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProperties {
    pub id: String,
    pub uid: u32,
    pub user: String,
    pub seat: String,
    #[serde(rename = "type")]
    pub session_type: String,
    pub class: String,
    pub active: bool,
    pub state: String,
    pub remote: bool,
    pub remote_host: String,
    pub remote_user: String,
    pub service: String,
    pub desktop: String,
    pub scope: String,
    pub leader: u32,
    pub audit: u32,
    pub vt_nr: u32,
    pub tty: String,
    pub display: String,
    pub timestamp: u64,
}

/// Detailed session status (textual, from session-status)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatus {
    pub id: String,
    pub properties: SessionProperties,
}

/// Summary information about a logged-in user (from ListUsers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub uid: u32,
    pub name: String,
    pub path: String,
}

/// Detailed user properties (from GetUser / show-user)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProperties {
    pub uid: u32,
    pub name: String,
    pub state: String,
    pub linger: bool,
    pub runtime_path: String,
    pub service: String,
    pub slice: String,
    pub display: String,
    pub timestamp: u64,
    pub sessions: Vec<String>,
}

/// Detailed user status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStatus {
    pub uid: u32,
    pub name: String,
    pub properties: UserProperties,
}

/// Request body for kill operations
#[derive(Debug, Clone, serde::Deserialize)]
pub struct KillRequest {
    #[serde(default = "default_signal")]
    pub signal: String,
    /// For kill-session: "leader", "all" etc.
    #[serde(default)]
    pub who: Option<String>,
}

fn default_signal() -> String {
    "SIGTERM".to_string()
}
