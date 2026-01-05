//! Application discovery and management.
//!
//! Parses .desktop files from XDG directories to build an app list.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// XDG application directories
const APP_DIRS: &[&str] = &[
    "/usr/share/applications",
    "/usr/local/share/applications",
    "~/.local/share/applications",
    "/var/lib/flatpak/exports/share/applications",
    "~/.local/share/flatpak/exports/share/applications",
];

/// An application entry
#[derive(Debug, Clone)]
pub struct App {
    /// Display name
    pub name: String,
    /// Generic name (optional)
    pub generic_name: Option<String>,
    /// Executable command
    pub exec: String,
    /// Icon name or path
    pub icon: Option<String>,
    /// Categories
    pub categories: Vec<String>,
    /// Keywords for searching
    pub keywords: Vec<String>,
    /// Whether this is a terminal application
    pub terminal: bool,
    /// Desktop file path
    pub desktop_file: PathBuf,
}

/// List of discovered applications
pub struct AppList {
    apps: Vec<App>,
    matcher: SkimMatcherV2,
}

impl AppList {
    /// Load applications from XDG directories
    pub fn load() -> Self {
        let mut apps = Vec::new();

        for dir_str in APP_DIRS {
            let dir = if dir_str.starts_with('~') {
                if let Some(home) = std::env::var_os("HOME") {
                    PathBuf::from(&home).join(&dir_str[2..])
                } else {
                    continue;
                }
            } else {
                PathBuf::from(dir_str)
            };

            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                        if let Some(app) = Self::parse_desktop_file(&path) {
                            apps.push(app);
                        }
                    }
                }
            }
        }

        // Sort by name
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // Remove duplicates (same name)
        apps.dedup_by(|a, b| a.name == b.name);

        Self {
            apps,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Parse a .desktop file
    fn parse_desktop_file(path: &PathBuf) -> Option<App> {
        let content = fs::read_to_string(path).ok()?;

        // Simple desktop file parser - only parse [Desktop Entry] section
        let mut in_desktop_entry = false;
        let mut attrs: HashMap<String, String> = HashMap::new();

        for line in content.lines() {
            let line = line.trim();

            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                in_desktop_entry = line == "[Desktop Entry]";
                continue;
            }

            // Key=Value pairs in Desktop Entry section
            if in_desktop_entry {
                if let Some((key, value)) = line.split_once('=') {
                    attrs.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }

        // Skip if NoDisplay or Hidden
        if attrs.get("NoDisplay").map(|v| v == "true").unwrap_or(false) {
            return None;
        }
        if attrs.get("Hidden").map(|v| v == "true").unwrap_or(false) {
            return None;
        }

        // Must have Name and Exec
        let name = attrs.get("Name")?.clone();
        let exec_raw = attrs.get("Exec")?.clone();

        // Clean up Exec (remove %f, %F, %u, %U, etc.)
        let exec = exec_raw
            .split_whitespace()
            .filter(|s| !s.starts_with('%'))
            .collect::<Vec<_>>()
            .join(" ");

        let generic_name = attrs.get("GenericName").cloned();
        let icon = attrs.get("Icon").cloned();
        let terminal = attrs
            .get("Terminal")
            .map(|v| v == "true")
            .unwrap_or(false);

        let categories = attrs
            .get("Categories")
            .map(|s| {
                s.split(';')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let keywords = attrs
            .get("Keywords")
            .map(|s| {
                s.split(';')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Some(App {
            name,
            generic_name,
            exec,
            icon,
            categories,
            keywords,
            terminal,
            desktop_file: path.clone(),
        })
    }

    /// Search apps with fuzzy matching
    pub fn search(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            // Return all apps
            return (0..self.apps.len()).collect();
        }

        let mut matches: Vec<(i64, usize)> = self
            .apps
            .iter()
            .enumerate()
            .filter_map(|(idx, app)| {
                // Match against name, generic name, and keywords
                let name_score = self.matcher.fuzzy_match(&app.name, query);
                let generic_score = app
                    .generic_name
                    .as_ref()
                    .and_then(|g| self.matcher.fuzzy_match(g, query));
                let keyword_score = app
                    .keywords
                    .iter()
                    .filter_map(|k| self.matcher.fuzzy_match(k, query))
                    .max();

                let best_score = [name_score, generic_score, keyword_score]
                    .into_iter()
                    .flatten()
                    .max()?;

                Some((best_score, idx))
            })
            .collect();

        // Sort by score (descending)
        matches.sort_by(|a, b| b.0.cmp(&a.0));

        matches.into_iter().map(|(_, idx)| idx).collect()
    }

    /// Get app by index
    pub fn get(&self, idx: usize) -> Option<&App> {
        self.apps.get(idx)
    }

    /// Number of apps
    pub fn len(&self) -> usize {
        self.apps.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_list_load() {
        let list = AppList::load();
        // Should load without panicking
        // May or may not find apps depending on environment
        assert!(list.len() >= 0);
    }

    #[test]
    fn test_search_empty() {
        let list = AppList::load();
        let results = list.search("");
        assert_eq!(results.len(), list.len());
    }
}
