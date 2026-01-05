//! osv-bar - Minimal status bar for osvwm
//!
//! A layer-shell based status bar showing:
//! - 8 fixed workspace indicators (circles for 1-6, diamonds for immersion 7-8)
//! - Clock display
//!
//! Uses the OSV color palette for consistent theming.

mod bar;
mod colors;
mod wayland;

use anyhow::Result;
use bar::BarState;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("osv-bar starting...");

    // Create bar state
    let bar_state = Rc::new(RefCell::new(BarState::default()));

    // TODO: Connect to osvwm via IPC to receive workspace updates
    // For now, we simulate some state for testing
    {
        let mut state = bar_state.borrow_mut();
        state.set_active(0);
        state.set_occupied(0, true);
        state.set_occupied(1, true);
        state.set_occupied(6, true); // Immersion workspace has a window
    }

    // Run the Wayland event loop
    wayland::run_bar(bar_state)?;

    info!("osv-bar exiting");
    Ok(())
}
