# Russessin

A REST API service that wraps `systemd-logind` (`loginctl`) operations, enabling remote session and user management over HTTP.

## Features

- Exposes 17 `loginctl` commands as RESTful endpoints (10 session + 7 user)
- Communicates with `systemd-logind` via D-Bus (`zbus` crate) for type-safe, native integration
- Runs as a `systemd` service
- Packaged as `.deb` for Ubuntu/Debian
- Configurable via TOML file

## API Reference

All responses are JSON. Mutating operations return `204 No Content` on success.

### Health Check

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/healthz` | Health check (returns `ok`) |

### Session Endpoints

| loginctl command | Method | Endpoint | Description |
|------------------|--------|----------|-------------|
| `list-sessions` | GET | `/api/v1/sessions` | List all sessions |
| `session-status ID` | GET | `/api/v1/sessions/{id}/status` | Session status details |
| `show-session ID` | GET | `/api/v1/sessions/{id}` | Session properties |
| `activate ID` | POST | `/api/v1/sessions/{id}/activate` | Activate a session |
| `lock-session ID` | POST | `/api/v1/sessions/{id}/lock` | Lock a session |
| `unlock-session ID` | POST | `/api/v1/sessions/{id}/unlock` | Unlock a session |
| `lock-sessions` | POST | `/api/v1/sessions/lock-all` | Lock all sessions |
| `unlock-sessions` | POST | `/api/v1/sessions/unlock-all` | Unlock all sessions |
| `terminate-session ID` | DELETE | `/api/v1/sessions/{id}` | Terminate a session |
| `kill-session ID` | POST | `/api/v1/sessions/{id}/kill` | Kill session processes |

### User Endpoints

| loginctl command | Method | Endpoint | Description |
|------------------|--------|----------|-------------|
| `list-users` | GET | `/api/v1/users` | List logged-in users |
| `user-status UID` | GET | `/api/v1/users/{uid}/status` | User status details |
| `show-user UID` | GET | `/api/v1/users/{uid}` | User properties |
| `enable-linger UID` | POST | `/api/v1/users/{uid}/linger` | Enable linger |
| `disable-linger UID` | DELETE | `/api/v1/users/{uid}/linger` | Disable linger |
| `terminate-user UID` | DELETE | `/api/v1/users/{uid}` | Terminate all user sessions |
| `kill-user UID` | POST | `/api/v1/users/{uid}/kill` | Kill user processes |

### Kill Request Body

For `kill-session` and `kill-user` endpoints:

```json
{
  "signal": "SIGTERM",
  "who": "all"
}
```

- `signal`: Signal name (e.g., `SIGTERM`, `SIGKILL`) or number. Default: `SIGTERM`
- `who` (session kill only): `"leader"` or `"all"`. Default: `"all"`

## Building

### Prerequisites

- Rust 1.75+ (with `cargo`)
- `libdbus-1-dev` (build dependency for D-Bus)

```bash
# Install build dependencies (Ubuntu/Debian)
sudo apt install libdbus-1-dev pkg-config

# Build
cargo build --release

# Run tests
cargo test
```

### Building the .deb Package

```bash
# Install cargo-deb
cargo install cargo-deb

# Build the .deb package
cargo deb
```

The resulting `.deb` file will be in `target/debian/`.

## Installation

### From .deb package

```bash
sudo dpkg -i target/debian/russessin_0.1.0-1_amd64.deb
```

### Configuration

Edit `/etc/russessin/russessin.toml`:

```toml
[server]
host = "127.0.0.1"
port = 3000

[logging]
level = "info"
```

### Running

```bash
# Start the service
sudo systemctl start russessin

# Enable on boot
sudo systemctl enable russessin

# Check status
sudo systemctl status russessin

# View logs
journalctl -u russessin -f
```

### Running manually

```bash
russessin --config /path/to/config.toml
```

## Examples

```bash
# List all sessions
curl http://localhost:3000/api/v1/sessions

# Get session properties
curl http://localhost:3000/api/v1/sessions/42

# List users
curl http://localhost:3000/api/v1/users

# Lock a session
curl -X POST http://localhost:3000/api/v1/sessions/42/lock

# Kill a user's processes
curl -X POST http://localhost:3000/api/v1/users/1000/kill \
  -H "Content-Type: application/json" \
  -d '{"signal": "SIGTERM"}'
```

## Future Work

- **JWT Bearer Authentication**: Protect endpoints with `Authorization: Bearer <jwt>` tokens
- **Seat commands**: Expose the remaining 6 `loginctl` seat commands

## License

MIT
