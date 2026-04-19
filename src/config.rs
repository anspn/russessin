use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
            path: path.display().to_string(),
            source: e,
        })?;
        let config: Config =
            toml::from_str(&content).map_err(|e| ConfigError::ParseError {
                path: path.display().to_string(),
                source: e,
            })?;
        Ok(config)
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file '{path}': {source}")]
    ReadError {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to parse config file '{path}': {source}")]
    ParseError {
        path: String,
        source: toml::de::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_valid_config() {
        let dir = std::env::temp_dir().join("russessin_test_valid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
[server]
host = "0.0.0.0"
port = 8080

[logging]
level = "debug"
"#
        )
        .unwrap();

        let config = Config::from_file(&path).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.bind_address(), "0.0.0.0:8080");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_missing_file() {
        let result = Config::from_file(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::ReadError { .. }));
    }

    #[test]
    fn test_invalid_toml() {
        let dir = std::env::temp_dir().join("russessin_test_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.toml");
        std::fs::write(&path, "this is not valid { toml").unwrap();

        let result = Config::from_file(&path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::ParseError { .. }));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        let dir = std::env::temp_dir().join("russessin_test_partial");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("partial.toml");
        std::fs::write(&path, "[server]\nport = 9090\n").unwrap();

        let config = Config::from_file(&path).unwrap();
        assert_eq!(config.server.host, "127.0.0.1"); // default
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.logging.level, "info"); // default

        std::fs::remove_dir_all(&dir).ok();
    }
}
