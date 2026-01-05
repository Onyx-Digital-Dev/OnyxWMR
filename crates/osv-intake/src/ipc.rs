//! IPC protocol for osv-intake-daemon communication.
//!
//! The daemon listens on a Unix socket and serves search queries.
//! Protocol: JSON-RPC style messages over Unix socket.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

/// Socket path for the daemon
pub const SOCKET_PATH: &str = "/tmp/osv-intake.sock";

/// Request from client to daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    /// Search for applications
    #[serde(rename = "search")]
    Search { query: String },
    /// Get all applications (empty query)
    #[serde(rename = "list")]
    List,
    /// Ping to check daemon status
    #[serde(rename = "ping")]
    Ping,
}

/// Response from daemon to client
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    /// Search results
    #[serde(rename = "results")]
    Results { apps: Vec<AppInfo> },
    /// Pong response
    #[serde(rename = "pong")]
    Pong,
    /// Error response
    #[serde(rename = "error")]
    Error { message: String },
}

/// Application info sent over IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub generic_name: Option<String>,
    pub exec: String,
    pub icon: Option<String>,
    pub terminal: bool,
    pub desktop_file: PathBuf,
}

/// Client for communicating with osv-intake-daemon
/// Used by osv-intake (client), not osv-intake-daemon
#[allow(dead_code)]
pub struct DaemonClient {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

#[allow(dead_code)]
impl DaemonClient {
    pub fn connect() -> anyhow::Result<Self> {
        let writer = UnixStream::connect(SOCKET_PATH)?;
        writer.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        writer.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
        let reader = BufReader::new(writer.try_clone()?);
        Ok(Self { reader, writer })
    }

    /// Send a request and receive response
    pub fn request(&mut self, req: &Request) -> anyhow::Result<Response> {
        // Send request as JSON line
        let mut json = serde_json::to_string(req)?;
        json.push('\n');
        self.writer.write_all(json.as_bytes())?;
        self.writer.flush()?;

        // Read response
        let mut line = String::new();
        self.reader.read_line(&mut line)?;

        let resp: Response = serde_json::from_str(&line)?;
        Ok(resp)
    }

    /// Search for applications
    pub fn search(&mut self, query: &str) -> anyhow::Result<Vec<AppInfo>> {
        let req = if query.is_empty() {
            Request::List
        } else {
            Request::Search {
                query: query.to_string(),
            }
        };

        match self.request(&req)? {
            Response::Results { apps } => Ok(apps),
            Response::Error { message } => anyhow::bail!("Daemon error: {}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Check if daemon is alive
    pub fn ping(&mut self) -> anyhow::Result<bool> {
        match self.request(&Request::Ping)? {
            Response::Pong => Ok(true),
            _ => Ok(false),
        }
    }
}
