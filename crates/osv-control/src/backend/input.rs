//! Input device settings backend
//!
//! Provides mouse and touchpad configuration using libinput settings.
//! For Wayland compositors, these settings are typically handled per-device.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Input device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputDeviceType {
    Mouse,
    Touchpad,
    Trackball,
    Tablet,
    Keyboard,
    Unknown,
}

/// Mouse/pointer settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseSettings {
    /// Pointer speed (-1.0 to 1.0, 0 is default)
    pub speed: f64,
    /// Natural scrolling (reversed scroll direction)
    pub natural_scroll: bool,
    /// Left-handed mode (swap left/right buttons)
    pub left_handed: bool,
    /// Middle button emulation
    pub middle_emulation: bool,
    /// Scroll method
    pub scroll_method: ScrollMethod,
    /// Acceleration profile
    pub accel_profile: AccelProfile,
}

impl Default for MouseSettings {
    fn default() -> Self {
        Self {
            speed: 0.0,
            natural_scroll: false,
            left_handed: false,
            middle_emulation: false,
            scroll_method: ScrollMethod::OnButtonDown,
            accel_profile: AccelProfile::Adaptive,
        }
    }
}

/// Touchpad settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchpadSettings {
    /// Pointer speed (-1.0 to 1.0, 0 is default)
    pub speed: f64,
    /// Tap to click
    pub tap_to_click: bool,
    /// Tap and drag
    pub tap_and_drag: bool,
    /// Tap drag lock
    pub tap_drag_lock: bool,
    /// Natural scrolling
    pub natural_scroll: bool,
    /// Two-finger scroll
    pub two_finger_scroll: bool,
    /// Edge scroll
    pub edge_scroll: bool,
    /// Disable while typing
    pub disable_while_typing: bool,
    /// Click method
    pub click_method: ClickMethod,
    /// Acceleration profile
    pub accel_profile: AccelProfile,
}

impl Default for TouchpadSettings {
    fn default() -> Self {
        Self {
            speed: 0.0,
            tap_to_click: true,
            tap_and_drag: true,
            tap_drag_lock: false,
            natural_scroll: true,
            two_finger_scroll: true,
            edge_scroll: false,
            disable_while_typing: true,
            click_method: ClickMethod::ButtonAreas,
            accel_profile: AccelProfile::Adaptive,
        }
    }
}

/// Scroll method for pointing devices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScrollMethod {
    /// No scrolling
    None,
    /// Two-finger scroll (touchpad)
    TwoFinger,
    /// Edge scrolling (touchpad)
    Edge,
    /// Button-based scrolling (mouse)
    #[default]
    OnButtonDown,
}

/// Click method for touchpads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClickMethod {
    /// Button areas (bottom of touchpad)
    #[default]
    ButtonAreas,
    /// Click fingers (1 finger = left, 2 = right, 3 = middle)
    ClickFinger,
}

/// Acceleration profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AccelProfile {
    /// Pointer acceleration
    #[default]
    Adaptive,
    /// No acceleration (flat)
    Flat,
}

/// Input device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDevice {
    /// Device name
    pub name: String,
    /// Device path (e.g., /dev/input/event0)
    pub path: String,
    /// Device type
    pub device_type: InputDeviceType,
    /// Device vendor ID
    pub vendor_id: Option<u16>,
    /// Device product ID
    pub product_id: Option<u16>,
    /// Whether device supports tap
    pub supports_tap: bool,
    /// Whether device supports natural scroll
    pub supports_natural_scroll: bool,
    /// Whether device supports left-handed mode
    pub supports_left_handed: bool,
}

/// Input settings backend
pub struct InputBackend {
    /// Cached device list
    devices: Vec<InputDevice>,
    /// Mouse settings
    pub mouse_settings: MouseSettings,
    /// Touchpad settings
    pub touchpad_settings: TouchpadSettings,
}

impl Default for InputBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBackend {
    /// Create a new input backend
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            mouse_settings: MouseSettings::default(),
            touchpad_settings: TouchpadSettings::default(),
        }
    }

    /// Enumerate input devices
    pub fn enumerate_devices(&mut self) -> Result<Vec<InputDevice>> {
        let mut devices = Vec::new();

        // Try libinput list-devices
        if let Ok(output) = Command::new("libinput").args(["list-devices"]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                devices = Self::parse_libinput_list(&stdout);
            }
        }

        // Fallback: scan /sys/class/input
        if devices.is_empty() {
            devices = Self::scan_sys_input()?;
        }

        self.devices = devices.clone();
        Ok(devices)
    }

    /// Parse libinput list-devices output
    fn parse_libinput_list(output: &str) -> Vec<InputDevice> {
        let mut devices = Vec::new();
        let mut current_device: Option<InputDevice> = None;

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("Device:") {
                // Save previous device
                if let Some(dev) = current_device.take() {
                    devices.push(dev);
                }

                // Start new device
                let name = line.strip_prefix("Device:").unwrap_or("").trim().to_string();
                current_device = Some(InputDevice {
                    name,
                    path: String::new(),
                    device_type: InputDeviceType::Unknown,
                    vendor_id: None,
                    product_id: None,
                    supports_tap: false,
                    supports_natural_scroll: false,
                    supports_left_handed: false,
                });
            }

            if let Some(ref mut dev) = current_device {
                if line.starts_with("Kernel:") {
                    dev.path = line.strip_prefix("Kernel:").unwrap_or("").trim().to_string();
                }

                // Detect device type from capabilities
                if line.contains("pointer") {
                    if dev.name.to_lowercase().contains("touchpad") {
                        dev.device_type = InputDeviceType::Touchpad;
                        dev.supports_tap = true;
                        dev.supports_natural_scroll = true;
                    } else if dev.name.to_lowercase().contains("trackball") {
                        dev.device_type = InputDeviceType::Trackball;
                    } else {
                        dev.device_type = InputDeviceType::Mouse;
                        dev.supports_natural_scroll = true;
                        dev.supports_left_handed = true;
                    }
                }

                if line.contains("Tap-to-click:") {
                    dev.supports_tap = true;
                }

                if line.contains("Natural Scroll:") {
                    dev.supports_natural_scroll = true;
                }

                if line.contains("Left-handed:") {
                    dev.supports_left_handed = true;
                }
            }
        }

        // Don't forget the last device
        if let Some(dev) = current_device {
            devices.push(dev);
        }

        devices
    }

    /// Scan /sys/class/input for devices
    fn scan_sys_input() -> Result<Vec<InputDevice>> {
        let mut devices = Vec::new();
        let input_dir = Path::new("/sys/class/input");

        if !input_dir.exists() {
            return Ok(devices);
        }

        for entry in fs::read_dir(input_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();

            if !name.starts_with("event") {
                continue;
            }

            let path = entry.path();
            let device_path = path.join("device");

            if !device_path.exists() {
                continue;
            }

            // Read device name
            let name_path = device_path.join("name");
            let device_name = fs::read_to_string(&name_path)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| name.clone());

            // Determine device type from name
            let device_type = if device_name.to_lowercase().contains("touchpad") {
                InputDeviceType::Touchpad
            } else if device_name.to_lowercase().contains("mouse") {
                InputDeviceType::Mouse
            } else if device_name.to_lowercase().contains("keyboard") {
                InputDeviceType::Keyboard
            } else {
                InputDeviceType::Unknown
            };

            devices.push(InputDevice {
                name: device_name,
                path: format!("/dev/input/{}", name),
                device_type,
                vendor_id: None,
                product_id: None,
                supports_tap: device_type == InputDeviceType::Touchpad,
                supports_natural_scroll: matches!(
                    device_type,
                    InputDeviceType::Mouse | InputDeviceType::Touchpad
                ),
                supports_left_handed: device_type == InputDeviceType::Mouse,
            });
        }

        Ok(devices)
    }

    /// Get mouse devices
    pub fn get_mice(&self) -> Vec<&InputDevice> {
        self.devices
            .iter()
            .filter(|d| d.device_type == InputDeviceType::Mouse)
            .collect()
    }

    /// Get touchpad devices
    pub fn get_touchpads(&self) -> Vec<&InputDevice> {
        self.devices
            .iter()
            .filter(|d| d.device_type == InputDeviceType::Touchpad)
            .collect()
    }

    /// Apply mouse settings
    /// Note: For osvwm (Wayland), these settings need to be applied through the compositor
    pub fn apply_mouse_settings(&self, settings: &MouseSettings) -> Result<()> {
        // For X11/Wayland hybrid systems, we could use xinput
        // For pure Wayland, this needs compositor integration

        // Store settings for compositor to use
        // In practice, osvwm would read these from a config file or IPC

        tracing::info!("Mouse settings updated: {:?}", settings);
        Ok(())
    }

    /// Apply touchpad settings
    pub fn apply_touchpad_settings(&self, settings: &TouchpadSettings) -> Result<()> {
        // Same as mouse - needs compositor integration for Wayland

        tracing::info!("Touchpad settings updated: {:?}", settings);
        Ok(())
    }

    /// Convert speed value (0.0-1.0 UI range) to libinput (-1.0 to 1.0)
    pub fn ui_speed_to_libinput(ui_speed: f32) -> f64 {
        // UI: 0.0 = slowest, 1.0 = fastest
        // libinput: -1.0 = slowest, 1.0 = fastest
        (ui_speed as f64 * 2.0) - 1.0
    }

    /// Convert libinput speed to UI range
    pub fn libinput_speed_to_ui(libinput_speed: f64) -> f32 {
        ((libinput_speed + 1.0) / 2.0) as f32
    }

    /// Get combined input settings for serialization
    pub fn get_all_settings(&self) -> HashMap<String, serde_json::Value> {
        let mut settings = HashMap::new();

        if let Ok(mouse_json) = serde_json::to_value(&self.mouse_settings) {
            settings.insert("mouse".to_string(), mouse_json);
        }

        if let Ok(touchpad_json) = serde_json::to_value(&self.touchpad_settings) {
            settings.insert("touchpad".to_string(), touchpad_json);
        }

        settings
    }

    /// Load settings from configuration
    pub fn load_settings(&mut self, config_path: &Path) -> Result<()> {
        let content = fs::read_to_string(config_path)?;
        let settings: HashMap<String, serde_json::Value> = serde_json::from_str(&content)?;

        if let Some(mouse) = settings.get("mouse") {
            self.mouse_settings = serde_json::from_value(mouse.clone())?;
        }

        if let Some(touchpad) = settings.get("touchpad") {
            self.touchpad_settings = serde_json::from_value(touchpad.clone())?;
        }

        Ok(())
    }

    /// Save settings to configuration
    pub fn save_settings(&self, config_path: &Path) -> Result<()> {
        let settings = self.get_all_settings();
        let content = serde_json::to_string_pretty(&settings)?;
        fs::write(config_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_conversion() {
        // UI 0.0 -> libinput -1.0
        assert!((InputBackend::ui_speed_to_libinput(0.0) - (-1.0)).abs() < 0.001);
        // UI 0.5 -> libinput 0.0
        assert!((InputBackend::ui_speed_to_libinput(0.5) - 0.0).abs() < 0.001);
        // UI 1.0 -> libinput 1.0
        assert!((InputBackend::ui_speed_to_libinput(1.0) - 1.0).abs() < 0.001);

        // Reverse
        assert!((InputBackend::libinput_speed_to_ui(-1.0) - 0.0).abs() < 0.001);
        assert!((InputBackend::libinput_speed_to_ui(0.0) - 0.5).abs() < 0.001);
        assert!((InputBackend::libinput_speed_to_ui(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_default_mouse_settings() {
        let settings = MouseSettings::default();
        assert_eq!(settings.speed, 0.0);
        assert!(!settings.natural_scroll);
        assert!(!settings.left_handed);
    }

    #[test]
    fn test_default_touchpad_settings() {
        let settings = TouchpadSettings::default();
        assert!(settings.tap_to_click);
        assert!(settings.natural_scroll);
        assert!(settings.two_finger_scroll);
    }
}
