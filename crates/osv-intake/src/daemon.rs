//! osv-intake-daemon - Background application indexing service
//!
//! Scans .desktop files at startup, builds a searchable index,
//! and serves queries via Unix socket IPC.
//!
//! Started by the compositor and runs persistently.

mod apps;
mod protocol;

use anyhow::Result;
use apps::AppList;
use protocol::{AppInfo, Request, Response, SOCKET_PATH};
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

    // Handle connections - spawn threads for concurrent handling
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let apps_clone = Arc::clone(&apps);
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(stream, &apps_clone) {
                        // Only log if it's not a clean disconnect
                        if !e.to_string().contains("end of file") {
                            warn!("Client error: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                error!("Connection error: {}", e);
            }
        }
    }

    info!("osv-intake-daemon exiting");
    Ok(())
}

/// Handle a client connection - supports persistent connections with multiple requests
fn handle_client(stream: UnixStream, apps: &AppList) -> Result<()> {
    let reader_stream = stream.try_clone()?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;

    loop {
        // Read request line
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;

        // Clean disconnect
        if bytes_read == 0 {
            return Ok(());
        }

        // Skip empty lines
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse request
        let request: Request = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                // Send error response and continue
                let err_resp = Response::Error {
                    message: format!("Invalid JSON: {}", e),
                };
                let mut json = serde_json::to_string(&err_resp)?;
                json.push('\n');
                writer.write_all(json.as_bytes())?;
                writer.flush()?;
                continue;
            }
        };

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
    }
}
