//! Display management backend
//!
//! Provides display configuration including resolution, refresh rate,
//! orientation, and multi-monitor arrangement.
//! Works with wlr-randr for Wayland compositors.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Display/output information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Display {
    /// Output name (e.g., "DP-1", "HDMI-A-1")
    pub name: String,
    /// Make/manufacturer
    pub make: String,
    /// Model name
    pub model: String,
    /// Serial number
    pub serial: String,
    /// Whether this output is enabled
    pub enabled: bool,
    /// Current mode (resolution and refresh rate)
    pub current_mode: Option<DisplayMode>,
    /// Available modes
    pub modes: Vec<DisplayMode>,
    /// Current position
    pub position: Position,
    /// Current transform/rotation
    pub transform: Transform,
    /// Current scale factor
    pub scale: f32,
    /// Physical size in mm
    pub physical_size: PhysicalSize,
}

/// Display mode (resolution + refresh rate)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayMode {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Refresh rate in Hz
    pub refresh: f32,
    /// Whether this is the preferred mode
    pub preferred: bool,
}

/// Position on the virtual screen
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Physical dimensions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhysicalSize {
    pub width_mm: u32,
    pub height_mm: u32,
}

/// Display transform/rotation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum Transform {
    #[default]
    Normal,
    Rotate90,
    Rotate180,
    Rotate270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}

impl Transform {
    pub fn to_wlr_randr(&self) -> &'static str {
        match self {
            Transform::Normal => "normal",
            Transform::Rotate90 => "90",
            Transform::Rotate180 => "180",
            Transform::Rotate270 => "270",
            Transform::Flipped => "flipped",
            Transform::Flipped90 => "flipped-90",
            Transform::Flipped180 => "flipped-180",
            Transform::Flipped270 => "flipped-270",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "90" | "rotate90" => Transform::Rotate90,
            "180" | "rotate180" => Transform::Rotate180,
            "270" | "rotate270" => Transform::Rotate270,
            "flipped" => Transform::Flipped,
            "flipped-90" | "flipped90" => Transform::Flipped90,
            "flipped-180" | "flipped180" => Transform::Flipped180,
            "flipped-270" | "flipped270" => Transform::Flipped270,
            _ => Transform::Normal,
        }
    }
}

/// Wallpaper info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallpaper {
    /// Path to the wallpaper image
    pub path: PathBuf,
    /// Display mode
    pub mode: WallpaperMode,
}

/// How to display the wallpaper
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum WallpaperMode {
    #[default]
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
}

/// Display backend
pub struct DisplayBackend;

impl DisplayBackend {
    /// List all connected displays
    pub fn list_displays() -> Result<Vec<Display>> {
        let output = Command::new("wlr-randr")
            .arg("--json")
            .output()
            .context("Failed to run wlr-randr. Is it installed?")?;

        if !output.status.success() {
            // Try parsing text output as fallback
            return Self::list_displays_text();
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let displays: Vec<Display> =
            serde_json::from_str(&stdout).context("Failed to parse wlr-randr JSON output")?;

        Ok(displays)
    }

    /// List displays using text output (fallback)
    fn list_displays_text() -> Result<Vec<Display>> {
        let output = Command::new("wlr-randr")
            .output()
            .context("Failed to run wlr-randr")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut displays = Vec::new();
        let mut current_display: Option<Display> = None;
        let mut in_modes = false;

        for line in stdout.lines() {
            let trimmed = line.trim();

            // New output starts without leading whitespace
            if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
                // Save previous display
                if let Some(display) = current_display.take() {
                    displays.push(display);
                }

                // Parse output name and description
                // Format: "DP-1 "Make Model Serial""
                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                let name = parts[0].to_string();
                let description = parts.get(1).unwrap_or(&"").trim_matches('"');

                let desc_parts: Vec<&str> = description.split(' ').collect();
                let make = desc_parts.first().unwrap_or(&"Unknown").to_string();
                let model = desc_parts.get(1).unwrap_or(&"Unknown").to_string();
                let serial = desc_parts.get(2).unwrap_or(&"").to_string();

                current_display = Some(Display {
                    name,
                    make,
                    model,
                    serial,
                    enabled: true,
                    current_mode: None,
                    modes: Vec::new(),
                    position: Position::default(),
                    transform: Transform::Normal,
                    scale: 1.0,
                    physical_size: PhysicalSize::default(),
                });
                in_modes = false;
            } else if let Some(ref mut display) = current_display {
                // Parse display properties
                if trimmed.starts_with("Enabled:") {
                    display.enabled = trimmed.contains("yes");
                } else if trimmed.starts_with("Position:") {
                    if let Some(pos) = trimmed.strip_prefix("Position:") {
                        let coords: Vec<&str> = pos.trim().split(',').collect();
                        if coords.len() == 2 {
                            display.position.x = coords[0].trim().parse().unwrap_or(0);
                            display.position.y = coords[1].trim().parse().unwrap_or(0);
                        }
                    }
                } else if trimmed.starts_with("Transform:") {
                    if let Some(t) = trimmed.strip_prefix("Transform:") {
                        display.transform = Transform::from_str(t.trim());
                    }
                } else if trimmed.starts_with("Scale:") {
                    if let Some(s) = trimmed.strip_prefix("Scale:") {
                        display.scale = s.trim().parse().unwrap_or(1.0);
                    }
                } else if trimmed.starts_with("Physical size:") {
                    if let Some(size) = trimmed.strip_prefix("Physical size:") {
                        let parts: Vec<&str> = size.trim().split('x').collect();
                        if parts.len() == 2 {
                            display.physical_size.width_mm =
                                parts[0].trim().trim_end_matches("mm").parse().unwrap_or(0);
                            display.physical_size.height_mm = parts[1]
                                .trim()
                                .trim_end_matches("mm")
                                .trim()
                                .parse()
                                .unwrap_or(0);
                        }
                    }
                } else if trimmed == "Modes:" {
                    in_modes = true;
                } else if in_modes && trimmed.contains("px") {
                    // Parse mode: "1920x1080 px, 60.000000 Hz (preferred, current)"
                    if let Some(mode) = Self::parse_mode_line(trimmed) {
                        let is_current = trimmed.contains("current");
                        if is_current {
                            display.current_mode = Some(mode.clone());
                        }
                        display.modes.push(mode);
                    }
                }
            }
        }

        // Don't forget the last display
        if let Some(display) = current_display {
            displays.push(display);
        }

        Ok(displays)
    }

    /// Parse a mode line from wlr-randr text output
    fn parse_mode_line(line: &str) -> Option<DisplayMode> {
        // Format: "1920x1080 px, 60.000000 Hz (preferred, current)"
        let parts: Vec<&str> = line.split(',').collect();
        if parts.is_empty() {
            return None;
        }

        // Parse resolution
        let res_part = parts[0].trim().trim_end_matches(" px");
        let res: Vec<&str> = res_part.split('x').collect();
        if res.len() != 2 {
            return None;
        }

        let width = res[0].trim().parse().ok()?;
        let height = res[1].trim().parse().ok()?;

        // Parse refresh rate
        let refresh = parts
            .get(1)
            .and_then(|s| {
                s.trim()
                    .trim_end_matches(" Hz")
                    .trim()
                    .parse::<f32>()
                    .ok()
            })
            .unwrap_or(60.0);

        let preferred = line.contains("preferred");

        Some(DisplayMode {
            width,
            height,
            refresh,
            preferred,
        })
    }

    /// Set display mode (resolution and refresh rate)
    pub fn set_mode(output: &str, mode: &DisplayMode) -> Result<()> {
        let mode_str = format!("{}x{}@{}", mode.width, mode.height, mode.refresh);

        let output_cmd = Command::new("wlr-randr")
            .args(["--output", output, "--mode", &mode_str])
            .output()
            .context("Failed to set display mode")?;

        if !output_cmd.status.success() {
            anyhow::bail!(
                "Failed to set mode: {}",
                String::from_utf8_lossy(&output_cmd.stderr)
            );
        }

        Ok(())
    }

    /// Set display position
    pub fn set_position(output: &str, x: i32, y: i32) -> Result<()> {
        let pos_str = format!("{},{}", x, y);

        let output_cmd = Command::new("wlr-randr")
            .args(["--output", output, "--pos", &pos_str])
            .output()
            .context("Failed to set display position")?;

        if !output_cmd.status.success() {
            anyhow::bail!(
                "Failed to set position: {}",
                String::from_utf8_lossy(&output_cmd.stderr)
            );
        }

        Ok(())
    }

    /// Set display transform/rotation
    pub fn set_transform(output: &str, transform: Transform) -> Result<()> {
        let output_cmd = Command::new("wlr-randr")
            .args(["--output", output, "--transform", transform.to_wlr_randr()])
            .output()
            .context("Failed to set display transform")?;

        if !output_cmd.status.success() {
            anyhow::bail!(
                "Failed to set transform: {}",
                String::from_utf8_lossy(&output_cmd.stderr)
            );
        }

        Ok(())
    }

    /// Set display scale
    pub fn set_scale(output: &str, scale: f32) -> Result<()> {
        let scale_str = format!("{}", scale);

        let output_cmd = Command::new("wlr-randr")
            .args(["--output", output, "--scale", &scale_str])
            .output()
            .context("Failed to set display scale")?;

        if !output_cmd.status.success() {
            anyhow::bail!(
                "Failed to set scale: {}",
                String::from_utf8_lossy(&output_cmd.stderr)
            );
        }

        Ok(())
    }

    /// Enable or disable a display
    pub fn set_enabled(output: &str, enabled: bool) -> Result<()> {
        let flag = if enabled { "--on" } else { "--off" };

        let output_cmd = Command::new("wlr-randr")
            .args(["--output", output, flag])
            .output()
            .context("Failed to toggle display")?;

        if !output_cmd.status.success() {
            anyhow::bail!(
                "Failed to {} display: {}",
                if enabled { "enable" } else { "disable" },
                String::from_utf8_lossy(&output_cmd.stderr)
            );
        }

        Ok(())
    }

    /// List available wallpapers from common directories
    pub fn list_wallpapers() -> Result<Vec<PathBuf>> {
        let mut wallpapers = Vec::new();

        // Common wallpaper directories
        let dirs = [
            "/usr/share/backgrounds",
            "/usr/share/wallpapers",
            "~/.local/share/wallpapers",
            "~/.local/share/backgrounds",
        ];

        for dir in &dirs {
            let expanded = shellexpand::tilde(dir);
            let path = Path::new(expanded.as_ref());

            if path.exists() {
                Self::find_images_recursive(path, &mut wallpapers, 2)?;
            }
        }

        // Sort by filename
        wallpapers.sort_by(|a, b| {
            a.file_name()
                .cmp(&b.file_name())
        });

        Ok(wallpapers)
    }

    /// Recursively find image files
    fn find_images_recursive(dir: &Path, images: &mut Vec<PathBuf>, depth: u32) -> Result<()> {
        if depth == 0 {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).context("Failed to read directory")?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                Self::find_images_recursive(&path, images, depth - 1)?;
            } else if path.is_file() {
                // Check if it's an image by extension
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp") {
                        images.push(path);
                    }
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Brightness Control
// ============================================================================

/// Brightness information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrightnessInfo {
    /// Current brightness level (0-100)
    pub level: u8,
    /// Maximum brightness value (raw)
    pub max_brightness: u32,
    /// Current brightness value (raw)
    pub current_brightness: u32,
    /// Backlight device name
    pub device: String,
}

/// Brightness backend
pub struct BrightnessBackend;

impl BrightnessBackend {
    /// Get current brightness
    pub fn get() -> Result<BrightnessInfo> {
        // Try brightnessctl first (most reliable)
        if let Ok(info) = Self::get_brightnessctl() {
            return Ok(info);
        }

        // Fallback to reading /sys/class/backlight directly
        Self::get_sysfs()
    }

    /// Get brightness using brightnessctl
    fn get_brightnessctl() -> Result<BrightnessInfo> {
        let output = Command::new("brightnessctl")
            .args(["--machine-readable"])
            .output()
            .context("Failed to run brightnessctl")?;

        if !output.status.success() {
            anyhow::bail!("brightnessctl failed");
        }

        // Format: device,class,current,percentage,max
        // e.g., "intel_backlight,backlight,1000,50%,2000"
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split(',').collect();

        if parts.len() >= 5 {
            let device = parts[0].to_string();
            let current: u32 = parts[2].parse().unwrap_or(0);
            let percentage = parts[3].trim_end_matches('%').parse().unwrap_or(50);
            let max: u32 = parts[4].parse().unwrap_or(100);

            Ok(BrightnessInfo {
                level: percentage,
                max_brightness: max,
                current_brightness: current,
                device,
            })
        } else {
            anyhow::bail!("Failed to parse brightnessctl output");
        }
    }

    /// Get brightness from /sys/class/backlight
    fn get_sysfs() -> Result<BrightnessInfo> {
        let backlight_dir = Path::new("/sys/class/backlight");

        if !backlight_dir.exists() {
            anyhow::bail!("No backlight devices found");
        }

        // Find first backlight device
        let device = fs::read_dir(backlight_dir)?
            .filter_map(|e| e.ok())
            .next()
            .context("No backlight devices found")?;

        let device_path = device.path();
        let device_name = device.file_name().to_string_lossy().to_string();

        let max_brightness: u32 = fs::read_to_string(device_path.join("max_brightness"))?
            .trim()
            .parse()
            .context("Failed to parse max_brightness")?;

        let current_brightness: u32 = fs::read_to_string(device_path.join("brightness"))?
            .trim()
            .parse()
            .context("Failed to parse brightness")?;

        let level = if max_brightness > 0 {
            ((current_brightness as f32 / max_brightness as f32) * 100.0).round() as u8
        } else {
            0
        };

        Ok(BrightnessInfo {
            level,
            max_brightness,
            current_brightness,
            device: device_name,
        })
    }

    /// Set brightness level (0-100)
    pub fn set(level: u8) -> Result<()> {
        let level = level.min(100);

        // Try brightnessctl first
        if Self::set_brightnessctl(level).is_ok() {
            return Ok(());
        }

        // Fallback to sysfs
        Self::set_sysfs(level)
    }

    /// Set brightness using brightnessctl
    fn set_brightnessctl(level: u8) -> Result<()> {
        let level_str = format!("{}%", level);

        let output = Command::new("brightnessctl")
            .args(["set", &level_str])
            .output()
            .context("Failed to run brightnessctl")?;

        if !output.status.success() {
            anyhow::bail!(
                "brightnessctl failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Set brightness via sysfs (may require root)
    fn set_sysfs(level: u8) -> Result<()> {
        let backlight_dir = Path::new("/sys/class/backlight");

        let device = fs::read_dir(backlight_dir)?
            .filter_map(|e| e.ok())
            .next()
            .context("No backlight devices found")?;

        let device_path = device.path();

        let max_brightness: u32 = fs::read_to_string(device_path.join("max_brightness"))?
            .trim()
            .parse()
            .context("Failed to parse max_brightness")?;

        let new_value = ((level as f32 / 100.0) * max_brightness as f32).round() as u32;
        let new_value = new_value.max(1); // Ensure we don't set to 0

        fs::write(device_path.join("brightness"), new_value.to_string())
            .context("Failed to write brightness (may need root)")?;

        Ok(())
    }

    /// Increase brightness by a percentage
    pub fn increase(amount: u8) -> Result<()> {
        let current = Self::get()?.level;
        let new_level = current.saturating_add(amount).min(100);
        Self::set(new_level)
    }

    /// Decrease brightness by a percentage
    pub fn decrease(amount: u8) -> Result<()> {
        let current = Self::get()?.level;
        let new_level = current.saturating_sub(amount).max(1); // Don't go below 1%
        Self::set(new_level)
    }
}

// ============================================================================
// Night Mode / Color Temperature
// ============================================================================

/// Night mode status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightModeStatus {
    /// Whether night mode is currently active
    pub enabled: bool,
    /// Current color temperature in Kelvin (e.g., 4500K)
    pub temperature: Option<u32>,
    /// Whether running in automatic mode (based on time/location)
    pub automatic: bool,
}

/// Night mode backend
/// Supports gammastep, wlsunset, and redshift
pub struct NightModeBackend;

impl NightModeBackend {
    /// Get current night mode status
    pub fn status() -> Result<NightModeStatus> {
        // Check if gammastep is running
        if Self::is_process_running("gammastep") {
            return Ok(NightModeStatus {
                enabled: true,
                temperature: Self::get_gammastep_temp(),
                automatic: true,
            });
        }

        // Check if wlsunset is running
        if Self::is_process_running("wlsunset") {
            return Ok(NightModeStatus {
                enabled: true,
                temperature: None, // wlsunset doesn't expose current temp easily
                automatic: true,
            });
        }

        Ok(NightModeStatus {
            enabled: false,
            temperature: None,
            automatic: false,
        })
    }

    /// Check if a process is running
    fn is_process_running(name: &str) -> bool {
        Command::new("pgrep")
            .arg("-x")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get gammastep temperature from its status
    fn get_gammastep_temp() -> Option<u32> {
        // gammastep doesn't have a simple way to query current temp
        // Could parse from config or use DBus, but for now return None
        None
    }

    /// Enable night mode with specified temperature
    pub fn enable(temperature: u32) -> Result<()> {
        // Kill any existing night mode processes
        let _ = Self::disable();

        // Clamp temperature to reasonable range (1000K - 10000K)
        let temp = temperature.clamp(1000, 10000);

        // Try gammastep first (most common on Wayland)
        if Self::try_gammastep(temp).is_ok() {
            return Ok(());
        }

        // Try wlsunset
        if Self::try_wlsunset(temp).is_ok() {
            return Ok(());
        }

        anyhow::bail!("No night mode tool found. Install gammastep or wlsunset.");
    }

    /// Enable night mode with automatic scheduling
    pub fn enable_auto() -> Result<()> {
        // Kill any existing
        let _ = Self::disable();

        // Try gammastep in automatic mode
        if Command::new("gammastep")
            .spawn()
            .map(|_| ())
            .is_ok()
        {
            return Ok(());
        }

        // Try wlsunset in automatic mode (needs latitude/longitude or uses defaults)
        if Command::new("wlsunset")
            .spawn()
            .map(|_| ())
            .is_ok()
        {
            return Ok(());
        }

        anyhow::bail!("No night mode tool found. Install gammastep or wlsunset.");
    }

    /// Try to start gammastep with fixed temperature
    fn try_gammastep(temperature: u32) -> Result<()> {
        Command::new("gammastep")
            .args(["-O", &temperature.to_string()])
            .spawn()
            .context("Failed to start gammastep")?;
        Ok(())
    }

    /// Try to start wlsunset with fixed temperature
    fn try_wlsunset(temperature: u32) -> Result<()> {
        // wlsunset uses -T for high temp (day) and -t for low temp (night)
        // For fixed temp, we set both to the same value
        Command::new("wlsunset")
            .args(["-T", &temperature.to_string(), "-t", &temperature.to_string()])
            .spawn()
            .context("Failed to start wlsunset")?;
        Ok(())
    }

    /// Disable night mode
    pub fn disable() -> Result<()> {
        // Kill gammastep
        let _ = Command::new("pkill")
            .args(["-x", "gammastep"])
            .output();

        // Kill wlsunset
        let _ = Command::new("pkill")
            .args(["-x", "wlsunset"])
            .output();

        // Reset gamma using gammastep (if available)
        let _ = Command::new("gammastep")
            .args(["-x"])
            .output();

        Ok(())
    }

    /// Set color temperature (enables if not already enabled)
    pub fn set_temperature(temperature: u32) -> Result<()> {
        Self::enable(temperature)
    }

    /// Toggle night mode on/off
    pub fn toggle(temperature: u32) -> Result<()> {
        let status = Self::status()?;
        if status.enabled {
            Self::disable()
        } else {
            Self::enable(temperature)
        }
    }
}

/// Common night mode presets
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NightModePreset {
    /// Daylight (6500K) - neutral, no filtering
    Daylight,
    /// Sunset (4500K) - warm, moderate filtering
    Sunset,
    /// Candle (3000K) - very warm, strong filtering
    Candle,
    /// Night (2700K) - warmest, maximum filtering
    Night,
}

impl NightModePreset {
    /// Get the temperature in Kelvin for this preset
    pub fn temperature(&self) -> u32 {
        match self {
            NightModePreset::Daylight => 6500,
            NightModePreset::Sunset => 4500,
            NightModePreset::Candle => 3000,
            NightModePreset::Night => 2700,
        }
    }
}
