//! osv-intake-daemon - Background application indexing service
//!
//! Scans .desktop files at startup, builds a searchable index,
//! and serves queries via Unix socket IPC.
//!
//! Started by the compositor and runs persistently.

mod apps;
mod ipc;

use anyhow::Result;
use apps::AppList;
use ipc::{AppInfo, Request, Response, SOCKET_PATH};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("osv-intake-daemon starting...");

    // Remove stale socket if exists
    if std::path::Path::new(SOCKET_PATH).exists() {
        fs::remove_file(SOCKET_PATH)?;
    }

    // Load and index applications
    let apps = Arc::new(AppList::load());
    info!("Indexed {} applications", apps.len());

    // Create Unix socket listener
    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!("Listening on {}", SOCKET_PATH);

    // Handle connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let apps_clone = Arc::clone(&apps);
                // Handle each connection (could spawn thread for concurrency)
                if let Err(e) = handle_client(stream, &apps_clone) {
                    warn!("Client error: {}", e);
                }
            }
            Err(e) => {
                error!("Connection error: {}", e);
            }
        }
    }

    info!("osv-intake-daemon exiting");
    Ok(())
}

/// Handle a client connection
fn handle_client(stream: UnixStream, apps: &AppList) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;

    // Read request line
    let mut line = String::new();
    reader.read_line(&mut line)?;

    // Parse request
    let request: Request = serde_json::from_str(&line)?;

    // Process request
    let response = match request {
        Request::Search { query } => {
            let results = apps.search(&query);
            let app_infos: Vec<AppInfo> = results
                .into_iter()
                .filter_map(|idx| apps.get(idx))
                .map(|app| AppInfo {
                    name: app.name.clone(),
                    generic_name: app.generic_name.clone(),
                    exec: app.exec.clone(),
                    icon: app.icon.clone(),
                    terminal: app.terminal,
                    desktop_file: app.desktop_file.clone(),
                })
                .collect();
            Response::Results { apps: app_infos }
        }
        Request::List => {
            // Return all apps (limited to reasonable number)
            let app_infos: Vec<AppInfo> = (0..apps.len().min(100))
                .filter_map(|idx| apps.get(idx))
                .map(|app| AppInfo {
                    name: app.name.clone(),
                    generic_name: app.generic_name.clone(),
                    exec: app.exec.clone(),
                    icon: app.icon.clone(),
                    terminal: app.terminal,
                    desktop_file: app.desktop_file.clone(),
                })
                .collect();
            Response::Results { apps: app_infos }
        }
        Request::Ping => Response::Pong,
    };

    // Send response
    let mut json = serde_json::to_string(&response)?;
    json.push('\n');
    writer.write_all(json.as_bytes())?;
    writer.flush()?;

    Ok(())
}
