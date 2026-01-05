//! osv-bar - Minimal status bar for osvwm
//!
//! A layer-shell based status bar implementing Phase 2 directives:
//! - Workspace capsules with window icons (auto-sizing)
//! - "Onyx OSV" centered, time/date right
//! - Thin (24px), floating, 3D gradient + shadow
//! - Inter font, clean/professional
//! - IPC-driven updates from compositor

mod bar;
mod colors;
mod ipc;
mod text;
mod wayland;

use anyhow::Result;
use bar::{BarState, BAR_HEIGHT, BAR_MARGIN_TOP, SHADOW_BLUR};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Total height needed for the layer surface (bar + margin + shadow)
pub fn total_bar_height() -> u32 {
    BAR_HEIGHT + BAR_MARGIN_TOP + SHADOW_BLUR as u32 + 2
}

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("osv-bar starting...");
    info!(
        "Bar dimensions: {}px height, {}px total with margins/shadow",
        BAR_HEIGHT,
        total_bar_height()
    );

    // Check text rendering
    if text::is_available() {
        info!("Text rendering enabled");
    } else {
        warn!("Text rendering disabled (no font found)");
    }

    // Create bar state
    let bar_state = Rc::new(RefCell::new(BarState::default()));

    // Try to connect to osvwm IPC
    let ipc_client = match ipc::IpcClient::connect() {
        Ok(client) => {
            info!("Connected to osvwm IPC");
            Some(client)
        }
        Err(e) => {
            warn!("Failed to connect to osvwm IPC: {}", e);
            warn!("Running in standalone mode with simulated state");

            // Simulate some state for testing
            let mut state = bar_state.borrow_mut();
            state.set_active(0);
            state.set_window_count(0, 2);
            state.set_window_count(1, 1);
            state.set_window_count(6, 1);
            drop(state);

            None
        }
    };

    // Store IPC client in RefCell for access from Wayland callbacks
    let ipc_client = Rc::new(RefCell::new(ipc_client));

    // Run the Wayland event loop
    wayland::run_bar(bar_state, ipc_client)?;

    info!("osv-bar exiting");
    Ok(())
}
