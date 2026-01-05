//! osv-intake - Application launcher for osvwm
//!
//! A minimal, keyboard-driven application launcher using Slint UI.
//! Triggered by Super+Space, provides command input and app search.

mod apps;

use anyhow::Result;
use apps::AppList;
use slint::ComponentHandle;
use std::process::Command;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

slint::include_modules!();

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("osv-intake starting...");

    // Load applications for searching
    let apps = AppList::load();
    info!("Loaded {} applications", apps.len());

    // Create the Slint UI
    let ui = IntakeLauncher::new()?;

    // Handle search changes (for future app matching)
    let apps_for_search = apps;
    ui.on_search_changed(move |query| {
        let results = apps_for_search.search(&query);
        info!("Search '{}' matched {} apps", query, results.len());
    });

    // Handle command launch
    ui.on_launch_command(move |command| {
        let cmd = command.to_string();
        if !cmd.is_empty() {
            info!("Launching: {}", cmd);
            // Spawn the command
            if let Err(e) = Command::new("sh").arg("-c").arg(&cmd).spawn() {
                tracing::error!("Failed to launch '{}': {}", cmd, e);
            }
        }
        slint::quit_event_loop().ok();
    });

    // Handle close
    ui.on_close_launcher(|| {
        info!("Launcher closed");
        slint::quit_event_loop().ok();
    });

    // Run the event loop
    ui.run()?;

    info!("osv-intake exiting");
    Ok(())
}
