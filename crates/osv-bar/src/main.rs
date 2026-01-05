//! osv-bar - Minimal status bar for osvwm
//!
//! A Slint-based status bar implementing Phase 2 directives:
//! - Workspace capsules with window counts
//! - "Onyx OSV" centered, time/date right
//! - Thin (24px), floating, 3D gradient + shadow
//! - Inter font, clean/professional
//! - IPC-driven updates from compositor

mod ipc;

use anyhow::Result;
use chrono::Local;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

slint::include_modules!();

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("osv-bar starting...");

    // Create the Slint UI
    let ui = StatusBar::new()?;

    // Initialize window counts model
    let window_counts: Rc<VecModel<i32>> = Rc::new(VecModel::from(vec![0; 8]));
    ui.set_window_counts(ModelRc::from(window_counts.clone()));

    // Initialize urgent workspaces model
    let urgent_workspaces: Rc<VecModel<bool>> = Rc::new(VecModel::from(vec![false; 8]));
    ui.set_urgent_workspaces(ModelRc::from(urgent_workspaces.clone()));

    // Try to connect to osvwm IPC
    let ipc_client = match ipc::IpcClient::connect() {
        Ok(client) => {
            info!("Connected to osvwm IPC");
            Some(Arc::new(std::sync::Mutex::new(client)))
        }
        Err(e) => {
            warn!("Failed to connect to osvwm IPC: {}", e);
            warn!("Running in standalone mode with simulated state");

            // Simulate some state for testing
            window_counts.set_row_data(0, 2);
            window_counts.set_row_data(1, 1);
            window_counts.set_row_data(6, 1);

            None
        }
    };

    // Handle workspace switch
    let ipc_for_switch = ipc_client.clone();
    ui.on_switch_workspace(move |workspace| {
        info!("Switch to workspace {}", workspace);
        if let Some(ref client) = ipc_for_switch {
            if let Ok(mut c) = client.lock() {
                if let Err(e) = c.switch_workspace(workspace as usize) {
                    warn!("Failed to switch workspace: {}", e);
                }
            }
        }
    });

    // Update time periodically
    let ui_weak = ui.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(1),
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                let now = Local::now();
                ui.set_current_time(SharedString::from(now.format("%H:%M").to_string()));
                ui.set_current_date(SharedString::from(now.format("%a %b %d").to_string()));
            }
        },
    );

    // Set initial time
    let now = Local::now();
    ui.set_current_time(SharedString::from(now.format("%H:%M").to_string()));
    ui.set_current_date(SharedString::from(now.format("%a %b %d").to_string()));

    // Run the event loop
    ui.run()?;

    info!("osv-bar exiting");
    Ok(())
}
