//! osv-bar - Minimal status bar for osvwm
//!
//! A Slint-based status bar implementing Phase 2 directives:
//! - Workspace capsules with window indicators (auto-sizing)
//! - "Onyx OSV" centered, time/date right
//! - Thin (16px), floating, 3D gradient + shadow
//! - Inter font, clean/professional
//! - IPC event stream daemon for real-time updates

mod ipc;

use anyhow::Result;
use chrono::Local;
use ipc::{IpcCommandClient, IpcEventClient, IpcMessage, WORKSPACE_COUNT};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

slint::include_modules!();

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("osv-bar daemon starting...");

    // Create the Slint UI
    let ui = StatusBar::new()?;

    // Initialize models for reactive updates
    let window_counts: Rc<VecModel<i32>> = Rc::new(VecModel::from(vec![0i32; 8]));
    ui.set_window_counts(ModelRc::from(window_counts.clone()));

    let urgent_workspaces: Rc<VecModel<bool>> = Rc::new(VecModel::from(vec![false; 8]));
    ui.set_urgent_workspaces(ModelRc::from(urgent_workspaces.clone()));

    // Track workspace IDs for window counting
    let workspace_ids: Arc<Mutex<HashMap<u64, usize>>> = Arc::new(Mutex::new(HashMap::new()));

    // Connect to IPC event stream (daemon mode)
    let event_client = match IpcEventClient::connect() {
        Ok(client) => {
            info!("Connected to osvwm event stream (daemon mode)");
            Some(client)
        }
        Err(e) => {
            warn!("Failed to connect to osvwm event stream: {}", e);
            warn!("Running in standalone mode with simulated state");

            // Simulate some state for testing
            window_counts.set_row_data(0, 2);
            window_counts.set_row_data(1, 1);
            window_counts.set_row_data(6, 1);

            None
        }
    };

    // Connect to IPC for sending commands
    let cmd_client = match IpcCommandClient::connect() {
        Ok(client) => Some(Arc::new(Mutex::new(client))),
        Err(e) => {
            warn!("Failed to connect to osvwm command socket: {}", e);
            None
        }
    };

    // Handle workspace switch clicks
    let cmd_for_switch = cmd_client.clone();
    ui.on_switch_workspace(move |workspace| {
        info!("Switch to workspace {}", workspace + 1);
        if let Some(ref client) = cmd_for_switch {
            if let Ok(mut c) = client.lock() {
                if let Err(e) = c.switch_workspace(workspace as usize) {
                    warn!("Failed to switch workspace: {}", e);
                }
            }
        }
    });

    // IPC event polling timer (daemon loop)
    let ui_weak_ipc = ui.as_weak();
    let window_counts_ipc = window_counts.clone();
    let urgent_ws_ipc = urgent_workspaces.clone();
    let workspace_ids_ipc = workspace_ids.clone();

    let ipc_timer = slint::Timer::default();
    if let Some(event_client) = event_client {
        ipc_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(50), // Poll at 20Hz
            move || {
                while let Some(msg) = event_client.try_recv() {
                    match msg {
                        IpcMessage::WorkspacesChanged(workspaces) => {
                            // Update workspace ID mapping
                            if let Ok(mut ids) = workspace_ids_ipc.lock() {
                                ids.clear();
                                for ws in &workspaces {
                                    ids.insert(ws.id, ws.idx);
                                }
                            }

                            // Update active workspace and urgency
                            if let Some(ui) = ui_weak_ipc.upgrade() {
                                for ws in &workspaces {
                                    if ws.idx < WORKSPACE_COUNT {
                                        if ws.is_focused {
                                            ui.set_active_workspace(ws.idx as i32);
                                        }
                                        urgent_ws_ipc.set_row_data(ws.idx, ws.is_urgent);
                                    }
                                }
                            }
                        }

                        IpcMessage::WorkspaceActivated { idx } => {
                            if let Some(ui) = ui_weak_ipc.upgrade() {
                                ui.set_active_workspace(idx as i32);
                            }
                        }

                        IpcMessage::WindowsChanged(windows) => {
                            // Count windows per workspace
                            let mut counts = [0i32; WORKSPACE_COUNT];

                            if let Ok(ids) = workspace_ids_ipc.lock() {
                                for win in &windows {
                                    if let Some(ws_id) = win.workspace_id {
                                        if let Some(&idx) = ids.get(&ws_id) {
                                            if idx < WORKSPACE_COUNT {
                                                counts[idx] += 1;
                                            }
                                        }
                                    }
                                }
                            }

                            // Update UI
                            for (idx, count) in counts.iter().enumerate() {
                                window_counts_ipc.set_row_data(idx, *count);
                            }
                        }

                        IpcMessage::Disconnected => {
                            warn!("IPC connection lost");
                        }
                    }
                }
            },
        );
    }

    // Time update timer
    let ui_weak_time = ui.as_weak();
    let time_timer = slint::Timer::default();
    time_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(1),
        move || {
            if let Some(ui) = ui_weak_time.upgrade() {
                let now = Local::now();
                ui.set_current_time(SharedString::from(now.format("%H:%M").to_string()));
                ui.set_current_date(SharedString::from(now.format("%b %d").to_string()));
            }
        },
    );

    // Set initial time
    let now = Local::now();
    ui.set_current_time(SharedString::from(now.format("%H:%M").to_string()));
    ui.set_current_date(SharedString::from(now.format("%b %d").to_string()));

    info!("osv-bar daemon running");

    // Run the event loop
    ui.run()?;

    info!("osv-bar daemon exiting");
    Ok(())
}
