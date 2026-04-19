use async_trait::async_trait;
use crate::error::AppError;
use crate::logind::types::*;
use crate::logind::LogindClient;
use std::collections::HashMap;
use zbus::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

const LOGIND_BUS: &str = "org.freedesktop.login1";
const LOGIND_PATH: &str = "/org/freedesktop/login1";
const SESSION_IFACE: &str = "org.freedesktop.login1.Session";
const USER_IFACE: &str = "org.freedesktop.login1.User";

type PropMap = HashMap<String, OwnedValue>;

/// Real D-Bus client for systemd-logind via zbus.
pub struct ZbusLogindClient {
    connection: Connection,
}

impl ZbusLogindClient {
    pub async fn new() -> Result<Self, AppError> {
        let connection = Connection::system()
            .await
            .map_err(|e| AppError::Dbus(format!("failed to connect to system D-Bus: {e}")))?;
        Ok(Self { connection })
    }

    async fn manager_proxy(&self) -> Result<zbus::Proxy<'_>, AppError> {
        zbus::Proxy::new(
            &self.connection,
            LOGIND_BUS,
            LOGIND_PATH,
            "org.freedesktop.login1.Manager",
        )
        .await
        .map_err(|e| AppError::Dbus(format!("failed to create manager proxy: {e}")))
    }

    async fn make_proxy<'a>(&'a self, path: &'a str, iface: &'a str) -> Result<zbus::Proxy<'a>, AppError> {
        zbus::Proxy::new(&self.connection, LOGIND_BUS, path, iface)
            .await
            .map_err(|e| AppError::Dbus(format!("failed to create proxy for {iface}: {e}")))
    }

    async fn get_session_path(&self, id: &str) -> Result<String, AppError> {
        let proxy = self.manager_proxy().await?;
        let path: OwnedObjectPath = proxy
            .call("GetSession", &(id,))
            .await
            .map_err(|e| AppError::NotFound(format!("session '{id}' not found: {e}")))?;
        Ok(path.to_string())
    }

    async fn get_user_path(&self, uid: u32) -> Result<String, AppError> {
        let proxy = self.manager_proxy().await?;
        let path: OwnedObjectPath = proxy
            .call("GetUser", &(uid,))
            .await
            .map_err(|e| AppError::NotFound(format!("user {uid} not found: {e}")))?;
        Ok(path.to_string())
    }

    /// Fetch all properties of an interface via org.freedesktop.DBus.Properties.GetAll
    async fn get_all_properties(&self, path: &str, iface: &str) -> Result<PropMap, AppError> {
        let proxy = self
            .make_proxy(path, "org.freedesktop.DBus.Properties")
            .await?;
        let props: PropMap = proxy
            .call("GetAll", &(iface,))
            .await
            .map_err(|e| AppError::Dbus(format!("GetAll({iface}) failed: {e}")))?;
        Ok(props)
    }

    fn prop_string(props: &PropMap, key: &str) -> String {
        props
            .get(key)
            .and_then(|v| <String as TryFrom<OwnedValue>>::try_from(v.clone()).ok())
            .unwrap_or_default()
    }

    fn prop_bool(props: &PropMap, key: &str) -> bool {
        props
            .get(key)
            .and_then(|v| <bool as TryFrom<OwnedValue>>::try_from(v.clone()).ok())
            .unwrap_or(false)
    }

    fn prop_u32(props: &PropMap, key: &str) -> u32 {
        props
            .get(key)
            .and_then(|v| <u32 as TryFrom<OwnedValue>>::try_from(v.clone()).ok())
            .unwrap_or(0)
    }

    fn prop_u64(props: &PropMap, key: &str) -> u64 {
        props
            .get(key)
            .and_then(|v| <u64 as TryFrom<OwnedValue>>::try_from(v.clone()).ok())
            .unwrap_or(0)
    }

    fn prop_struct_first_u32(props: &PropMap, key: &str) -> u32 {
        props.get(key).and_then(|v| {
            let s = <zbus::zvariant::Structure as TryFrom<OwnedValue>>::try_from(v.clone()).ok()?;
            let fields = s.into_fields();
            fields.into_iter().next().and_then(|f| {
                <u32 as TryFrom<zbus::zvariant::Value>>::try_from(f).ok()
            })
        }).unwrap_or(0)
    }

    fn prop_struct_first_string(props: &PropMap, key: &str) -> String {
        props.get(key).and_then(|v| {
            let s = <zbus::zvariant::Structure as TryFrom<OwnedValue>>::try_from(v.clone()).ok()?;
            let fields = s.into_fields();
            fields.into_iter().next().and_then(|f| {
                <String as TryFrom<zbus::zvariant::Value>>::try_from(f).ok()
            })
        }).unwrap_or_default()
    }

    fn prop_sessions_list(props: &PropMap, key: &str) -> Vec<String> {
        props.get(key).and_then(|v| {
            let arr = <Vec<(String, OwnedObjectPath)> as TryFrom<OwnedValue>>::try_from(v.clone()).ok()?;
            Some(arr.into_iter().map(|(id, _)| id).collect())
        }).unwrap_or_default()
    }

    fn build_session_props_from_map(props: &PropMap) -> SessionProperties {
        SessionProperties {
            id: Self::prop_string(props, "Id"),
            uid: Self::prop_struct_first_u32(props, "User"),
            user: Self::prop_string(props, "Name"),
            seat: Self::prop_struct_first_string(props, "Seat"),
            session_type: Self::prop_string(props, "Type"),
            class: Self::prop_string(props, "Class"),
            active: Self::prop_bool(props, "Active"),
            state: Self::prop_string(props, "State"),
            remote: Self::prop_bool(props, "Remote"),
            remote_host: Self::prop_string(props, "RemoteHost"),
            remote_user: Self::prop_string(props, "RemoteUser"),
            service: Self::prop_string(props, "Service"),
            desktop: Self::prop_string(props, "Desktop"),
            scope: Self::prop_string(props, "Scope"),
            leader: Self::prop_u32(props, "Leader"),
            audit: Self::prop_u32(props, "Audit"),
            vt_nr: Self::prop_u32(props, "VTNr"),
            tty: Self::prop_string(props, "TTY"),
            display: Self::prop_string(props, "Display"),
            timestamp: Self::prop_u64(props, "Timestamp"),
        }
    }

    fn build_user_props_from_map(props: &PropMap) -> UserProperties {
        UserProperties {
            uid: Self::prop_u32(props, "UID"),
            name: Self::prop_string(props, "Name"),
            state: Self::prop_string(props, "State"),
            linger: Self::prop_bool(props, "Linger"),
            runtime_path: Self::prop_string(props, "RuntimePath"),
            service: Self::prop_string(props, "Service"),
            slice: Self::prop_string(props, "Slice"),
            display: Self::prop_struct_first_string(props, "Display"),
            timestamp: Self::prop_u64(props, "Timestamp"),
            sessions: Self::prop_sessions_list(props, "Sessions"),
        }
    }
}

#[async_trait]
impl LogindClient for ZbusLogindClient {
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, AppError> {
        let proxy = self.manager_proxy().await?;
        let sessions: Vec<(String, u32, String, String, OwnedObjectPath)> = proxy
            .call("ListSessions", &())
            .await
            .map_err(|e| AppError::Dbus(format!("ListSessions failed: {e}")))?;

        Ok(sessions
            .into_iter()
            .map(|(id, uid, user, seat, path)| SessionInfo {
                id,
                uid,
                user,
                seat,
                path: path.to_string(),
            })
            .collect())
    }

    async fn session_status(&self, id: &str) -> Result<SessionStatus, AppError> {
        let path = self.get_session_path(id).await?;
        let props = self.get_all_properties(&path, SESSION_IFACE).await?;
        Ok(SessionStatus {
            id: id.to_string(),
            properties: Self::build_session_props_from_map(&props),
        })
    }

    async fn show_session(&self, id: &str) -> Result<SessionProperties, AppError> {
        let path = self.get_session_path(id).await?;
        let props = self.get_all_properties(&path, SESSION_IFACE).await?;
        Ok(Self::build_session_props_from_map(&props))
    }

    async fn activate_session(&self, id: &str) -> Result<(), AppError> {
        let path = self.get_session_path(id).await?;
        let proxy = self.make_proxy(&path, SESSION_IFACE).await?;
        let _: () = proxy
            .call("Activate", &())
            .await
            .map_err(|e| AppError::Dbus(format!("Activate session '{id}' failed: {e}")))?;
        Ok(())
    }

    async fn lock_session(&self, id: &str) -> Result<(), AppError> {
        let path = self.get_session_path(id).await?;
        let proxy = self.make_proxy(&path, SESSION_IFACE).await?;
        let _: () = proxy
            .call("Lock", &())
            .await
            .map_err(|e| AppError::Dbus(format!("Lock session '{id}' failed: {e}")))?;
        Ok(())
    }

    async fn unlock_session(&self, id: &str) -> Result<(), AppError> {
        let path = self.get_session_path(id).await?;
        let proxy = self.make_proxy(&path, SESSION_IFACE).await?;
        let _: () = proxy
            .call("Unlock", &())
            .await
            .map_err(|e| AppError::Dbus(format!("Unlock session '{id}' failed: {e}")))?;
        Ok(())
    }

    async fn lock_sessions(&self) -> Result<(), AppError> {
        let proxy = self.manager_proxy().await?;
        let _: () = proxy
            .call("LockSessions", &())
            .await
            .map_err(|e| AppError::Dbus(format!("LockSessions failed: {e}")))?;
        Ok(())
    }

    async fn unlock_sessions(&self) -> Result<(), AppError> {
        let proxy = self.manager_proxy().await?;
        let _: () = proxy
            .call("UnlockSessions", &())
            .await
            .map_err(|e| AppError::Dbus(format!("UnlockSessions failed: {e}")))?;
        Ok(())
    }

    async fn terminate_session(&self, id: &str) -> Result<(), AppError> {
        let proxy = self.manager_proxy().await?;
        let _: () = proxy
            .call("TerminateSession", &(id,))
            .await
            .map_err(|e| AppError::Dbus(format!("TerminateSession '{id}' failed: {e}")))?;
        Ok(())
    }

    async fn kill_session(&self, id: &str, who: &str, signal: i32) -> Result<(), AppError> {
        let path = self.get_session_path(id).await?;
        let proxy = self.make_proxy(&path, SESSION_IFACE).await?;
        let _: () = proxy
            .call("Kill", &(who, signal))
            .await
            .map_err(|e| AppError::Dbus(format!("Kill session '{id}' failed: {e}")))?;
        Ok(())
    }

    async fn list_users(&self) -> Result<Vec<UserInfo>, AppError> {
        let proxy = self.manager_proxy().await?;
        let users: Vec<(u32, String, OwnedObjectPath)> = proxy
            .call("ListUsers", &())
            .await
            .map_err(|e| AppError::Dbus(format!("ListUsers failed: {e}")))?;

        Ok(users
            .into_iter()
            .map(|(uid, name, path)| UserInfo {
                uid,
                name,
                path: path.to_string(),
            })
            .collect())
    }

    async fn user_status(&self, uid: u32) -> Result<UserStatus, AppError> {
        let path = self.get_user_path(uid).await?;
        let props = self.get_all_properties(&path, USER_IFACE).await?;
        let user_props = Self::build_user_props_from_map(&props);
        Ok(UserStatus {
            uid,
            name: user_props.name.clone(),
            properties: user_props,
        })
    }

    async fn show_user(&self, uid: u32) -> Result<UserProperties, AppError> {
        let path = self.get_user_path(uid).await?;
        let props = self.get_all_properties(&path, USER_IFACE).await?;
        Ok(Self::build_user_props_from_map(&props))
    }

    async fn enable_linger(&self, uid: u32) -> Result<(), AppError> {
        let proxy = self.manager_proxy().await?;
        let _: () = proxy
            .call("SetUserLinger", &(uid, true, false))
            .await
            .map_err(|e| AppError::Dbus(format!("EnableLinger for user {uid} failed: {e}")))?;
        Ok(())
    }

    async fn disable_linger(&self, uid: u32) -> Result<(), AppError> {
        let proxy = self.manager_proxy().await?;
        let _: () = proxy
            .call("SetUserLinger", &(uid, false, false))
            .await
            .map_err(|e| AppError::Dbus(format!("DisableLinger for user {uid} failed: {e}")))?;
        Ok(())
    }

    async fn terminate_user(&self, uid: u32) -> Result<(), AppError> {
        let proxy = self.manager_proxy().await?;
        let _: () = proxy
            .call("TerminateUser", &(uid,))
            .await
            .map_err(|e| AppError::Dbus(format!("TerminateUser {uid} failed: {e}")))?;
        Ok(())
    }

    async fn kill_user(&self, uid: u32, signal: i32) -> Result<(), AppError> {
        let proxy = self.manager_proxy().await?;
        let _: () = proxy
            .call("KillUser", &(uid, signal))
            .await
            .map_err(|e| AppError::Dbus(format!("KillUser {uid} failed: {e}")))?;
        Ok(())
    }
}
