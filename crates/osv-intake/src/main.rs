//! osv-intake - Application launcher for osvwm
//!
//! A minimal, keyboard-driven application launcher using layer-shell.
//! Triggered by Super+Space, provides fuzzy-matching search over .desktop entries.

mod apps;
mod colors;
mod render;
mod text;

use anyhow::Result;
use apps::AppList;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Launcher state
pub struct IntakeState {
    /// Search query
    pub query: String,
    /// List of applications
    pub apps: AppList,
    /// Filtered results
    pub results: Vec<usize>,
    /// Selected index in results
    pub selected: usize,
    /// Whether launcher is visible
    pub visible: bool,
}

impl Default for IntakeState {
    fn default() -> Self {
        let apps = AppList::load();
        Self {
            query: String::new(),
            apps,
            results: Vec::new(),
            selected: 0,
            visible: true,
        }
    }
}

impl IntakeState {
    /// Update search results based on query
    pub fn update_search(&mut self) {
        self.results = self.apps.search(&self.query);
        self.selected = 0;
    }

    /// Move selection up
    pub fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn select_down(&mut self) {
        if !self.results.is_empty() && self.selected < self.results.len() - 1 {
            self.selected += 1;
        }
    }

    /// Get currently selected app
    pub fn selected_app(&self) -> Option<&apps::App> {
        self.results
            .get(self.selected)
            .and_then(|&idx| self.apps.get(idx))
    }

    /// Launch selected app
    pub fn launch_selected(&self) -> Option<String> {
        self.selected_app().map(|app| app.exec.clone())
    }

    /// Add character to query
    pub fn add_char(&mut self, c: char) {
        self.query.push(c);
        self.update_search();
    }

    /// Remove last character from query
    pub fn backspace(&mut self) {
        self.query.pop();
        self.update_search();
    }

    /// Clear query
    pub fn clear(&mut self) {
        self.query.clear();
        self.update_search();
    }
}

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("osv-intake starting...");

    // Create launcher state
    let state = Rc::new(RefCell::new(IntakeState::default()));

    // Log app count
    {
        let s = state.borrow();
        info!("Loaded {} applications", s.apps.len());
    }

    // Initialize with all apps shown
    state.borrow_mut().update_search();

    // Check text rendering
    if text::is_available() {
        info!("Text rendering enabled");
    } else {
        tracing::warn!("Text rendering disabled (no font found)");
    }

    // TODO: Run Wayland layer-shell UI
    // For now, just print the top results
    {
        let s = state.borrow();
        info!("Top 5 applications:");
        for (i, &idx) in s.results.iter().take(5).enumerate() {
            if let Some(app) = s.apps.get(idx) {
                info!("  {}. {} - {}", i + 1, app.name, app.exec);
            }
        }
    }

    info!("osv-intake: Layer-shell UI not yet implemented");
    info!("osv-intake exiting");
    Ok(())
}
