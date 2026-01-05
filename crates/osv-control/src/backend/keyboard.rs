//! Keyboard settings backend
//!
//! Provides keyboard layout management, key repeat settings, and input configuration.
//! Uses localectl (systemd) and xkbcomp for layout management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Keyboard layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardLayout {
    /// Layout code (e.g., "us", "gb", "de")
    pub code: String,
    /// Display name (e.g., "English (US)")
    pub name: String,
    /// Variant (e.g., "dvorak", "colemak")
    pub variant: Option<String>,
}

/// Key repeat settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRepeatSettings {
    /// Delay before key repeat starts (milliseconds)
    pub delay: u32,
    /// Key repeat rate (keys per second)
    pub rate: u32,
}

impl Default for KeyRepeatSettings {
    fn default() -> Self {
        Self {
            delay: 500,
            rate: 25,
        }
    }
}

/// Complete keyboard settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardSettings {
    /// Current keyboard layout
    pub layout: KeyboardLayout,
    /// Key repeat configuration
    pub repeat: KeyRepeatSettings,
    /// Caps lock indicator enabled
    pub caps_lock_indicator: bool,
    /// Num lock state on startup
    pub num_lock_on_startup: bool,
}

/// Keyboard settings backend
pub struct KeyboardBackend;

impl Default for KeyboardBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardBackend {
    /// Create a new keyboard backend
    pub fn new() -> Self {
        Self
    }

    /// Get current keyboard settings
    pub fn get_settings(&self) -> Result<KeyboardSettings> {
        let layout = self.get_current_layout()?;
        let repeat = self.get_repeat_settings()?;

        Ok(KeyboardSettings {
            layout,
            repeat,
            caps_lock_indicator: true, // Default - would read from config
            num_lock_on_startup: false,
        })
    }

    /// Get current keyboard layout
    pub fn get_current_layout(&self) -> Result<KeyboardLayout> {
        // Try localectl first
        let output = Command::new("localectl")
            .arg("status")
            .output()
            .context("Failed to run localectl")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Self::parse_localectl_output(&stdout);
        }

        // Fallback: try setxkbmap
        let output = Command::new("setxkbmap")
            .args(["-query"])
            .output()
            .context("Failed to run setxkbmap")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Self::parse_setxkbmap_output(&stdout);
        }

        // Default fallback
        Ok(KeyboardLayout {
            code: "us".to_string(),
            name: "English (US)".to_string(),
            variant: None,
        })
    }

    /// Parse localectl output for keyboard layout
    fn parse_localectl_output(output: &str) -> Result<KeyboardLayout> {
        let mut layout_code = String::from("us");
        let mut variant = None;

        for line in output.lines() {
            let line = line.trim();
            if line.starts_with("X11 Layout:") || line.starts_with("VC Keymap:") {
                if let Some(value) = line.split(':').nth(1) {
                    layout_code = value.trim().to_string();
                }
            }
            if line.starts_with("X11 Variant:") {
                if let Some(value) = line.split(':').nth(1) {
                    let v = value.trim();
                    if !v.is_empty() {
                        variant = Some(v.to_string());
                    }
                }
            }
        }

        let name = Self::layout_code_to_name(&layout_code, variant.as_deref());

        Ok(KeyboardLayout {
            code: layout_code,
            name,
            variant,
        })
    }

    /// Parse setxkbmap query output
    fn parse_setxkbmap_output(output: &str) -> Result<KeyboardLayout> {
        let mut layout_code = String::from("us");
        let mut variant = None;

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                match parts[0] {
                    "layout:" => layout_code = parts[1].to_string(),
                    "variant:" => variant = Some(parts[1].to_string()),
                    _ => {}
                }
            }
        }

        let name = Self::layout_code_to_name(&layout_code, variant.as_deref());

        Ok(KeyboardLayout {
            code: layout_code,
            name,
            variant,
        })
    }

    /// Convert layout code to human-readable name
    fn layout_code_to_name(code: &str, variant: Option<&str>) -> String {
        let base_name = match code {
            "us" => "English (US)",
            "gb" => "English (UK)",
            "de" => "German",
            "fr" => "French",
            "es" => "Spanish",
            "it" => "Italian",
            "pt" => "Portuguese",
            "ru" => "Russian",
            "jp" => "Japanese",
            "kr" => "Korean",
            "cn" => "Chinese",
            "br" => "Portuguese (Brazil)",
            "ca" => "Canadian",
            "ch" => "Swiss",
            "se" => "Swedish",
            "no" => "Norwegian",
            "dk" => "Danish",
            "fi" => "Finnish",
            "pl" => "Polish",
            "cz" => "Czech",
            "hu" => "Hungarian",
            "ro" => "Romanian",
            "tr" => "Turkish",
            "gr" => "Greek",
            "il" => "Hebrew",
            "ara" => "Arabic",
            "th" => "Thai",
            "vn" => "Vietnamese",
            _ => code,
        };

        match variant {
            Some("dvorak") => format!("{} (Dvorak)", base_name),
            Some("colemak") => format!("{} (Colemak)", base_name),
            Some("mac") => format!("{} (Mac)", base_name),
            Some("intl") => format!("{} (International)", base_name),
            Some(v) if !v.is_empty() => format!("{} ({})", base_name, v),
            _ => base_name.to_string(),
        }
    }

    /// Set keyboard layout (requires appropriate permissions)
    pub fn set_layout(&self, code: &str, variant: Option<&str>) -> Result<()> {
        let mut args = vec!["set-x11-keymap", code];
        if let Some(v) = variant {
            args.push(v);
        }

        let status = Command::new("localectl")
            .args(&args)
            .status()
            .context("Failed to run localectl")?;

        if !status.success() {
            // Fallback to setxkbmap for current session
            let mut args = vec!["-layout", code];
            if let Some(v) = variant {
                args.extend(["-variant", v]);
            }

            Command::new("setxkbmap")
                .args(&args)
                .status()
                .context("Failed to set keyboard layout")?;
        }

        Ok(())
    }

    /// Get key repeat settings
    pub fn get_repeat_settings(&self) -> Result<KeyRepeatSettings> {
        // Try xset for X11
        let output = Command::new("xset")
            .args(["q"])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(settings) = Self::parse_xset_output(&stdout) {
                    return Ok(settings);
                }
            }
        }

        // Default settings
        Ok(KeyRepeatSettings::default())
    }

    /// Parse xset query output for keyboard settings
    fn parse_xset_output(output: &str) -> Option<KeyRepeatSettings> {
        // Look for "auto repeat delay:  500    repeat rate:  33"
        for line in output.lines() {
            if line.contains("auto repeat delay:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut delay = 500;
                let mut rate = 25;

                for (i, part) in parts.iter().enumerate() {
                    if *part == "delay:" && i + 1 < parts.len() {
                        delay = parts[i + 1].parse().unwrap_or(500);
                    }
                    if *part == "rate:" && i + 1 < parts.len() {
                        rate = parts[i + 1].parse().unwrap_or(25);
                    }
                }

                return Some(KeyRepeatSettings { delay, rate });
            }
        }

        None
    }

    /// Set key repeat settings
    pub fn set_repeat_settings(&self, delay: u32, rate: u32) -> Result<()> {
        // Try xset for X11
        let status = Command::new("xset")
            .args(["r", "rate", &delay.to_string(), &rate.to_string()])
            .status();

        if let Ok(status) = status {
            if status.success() {
                return Ok(());
            }
        }

        // For Wayland, this would need compositor-specific handling
        // osvwm would need to implement repeat settings in its input handling

        Ok(())
    }

    /// List available keyboard layouts
    pub fn list_layouts(&self) -> Result<Vec<KeyboardLayout>> {
        let output = Command::new("localectl")
            .args(["list-x11-keymap-layouts"])
            .output()
            .context("Failed to list keyboard layouts")?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let layouts: Vec<KeyboardLayout> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|code| KeyboardLayout {
                code: code.to_string(),
                name: Self::layout_code_to_name(code, None),
                variant: None,
            })
            .collect();

        Ok(layouts)
    }

    /// List available variants for a layout
    pub fn list_variants(&self, layout: &str) -> Result<Vec<String>> {
        let output = Command::new("localectl")
            .args(["list-x11-keymap-variants", layout])
            .output()
            .context("Failed to list keyboard variants")?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let variants: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(variants)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_code_to_name() {
        assert_eq!(KeyboardBackend::layout_code_to_name("us", None), "English (US)");
        assert_eq!(KeyboardBackend::layout_code_to_name("de", None), "German");
        assert_eq!(
            KeyboardBackend::layout_code_to_name("us", Some("dvorak")),
            "English (US) (Dvorak)"
        );
    }

    #[test]
    fn test_default_repeat_settings() {
        let settings = KeyRepeatSettings::default();
        assert_eq!(settings.delay, 500);
        assert_eq!(settings.rate, 25);
    }
}
