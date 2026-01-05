//! IPC protocol types shared between daemon and client.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOCKET_PATH: &str = "/tmp/osv-intake.sock";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    #[serde(rename = "search")]
    Search { query: String },
    #[serde(rename = "list")]
    List,
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    #[serde(rename = "results")]
    Results { apps: Vec<AppInfo> },
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub generic_name: Option<String>,
    pub exec: String,
    pub icon: Option<String>,
    pub terminal: bool,
    pub desktop_file: PathBuf,
}
