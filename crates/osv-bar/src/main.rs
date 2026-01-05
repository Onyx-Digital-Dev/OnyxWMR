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
mod wayland;

use anyhow::Result;
use bar::{BarState, BAR_HEIGHT, BAR_MARGIN_TOP, SHADOW_BLUR};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{info, Level};
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

    // Create bar state
    let bar_state = Rc::new(RefCell::new(BarState::default()));

    // TODO: Connect to osvwm via IPC to receive workspace updates
    // For now, simulate some state for testing
    {
        let mut state = bar_state.borrow_mut();
        state.set_active(0);
        state.set_window_count(0, 2); // 2 windows on workspace 1
        state.set_window_count(1, 1); // 1 window on workspace 2
        state.set_window_count(6, 1); // 1 window on immersion workspace 7
    }

    // Run the Wayland event loop
    wayland::run_bar(bar_state)?;

    info!("osv-bar exiting");
    Ok(())
}
