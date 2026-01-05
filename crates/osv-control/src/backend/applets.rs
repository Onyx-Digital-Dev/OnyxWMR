//! System applets backend
//!
//! Provides status information for common bar applets:
//! - Battery status
//! - Volume/audio
//! - Network connectivity
//! - Bluetooth
//! - Date/time
//! - CPU/Memory usage

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Combined applet status for all common indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppletStatus {
    pub battery: Option<BatteryStatus>,
    pub audio: AudioStatus,
    pub network: NetworkIndicator,
    pub bluetooth: BluetoothStatus,
    pub datetime: DateTimeInfo,
    pub resources: ResourceUsage,
}

/// Battery status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryStatus {
    /// Charge percentage (0-100)
    pub percentage: u8,
    /// Current state
    pub state: BatteryState,
    /// Time to empty/full in seconds (if known)
    pub time_remaining: Option<u64>,
    /// Power draw in watts (if known)
    pub power_draw: Option<f32>,
    /// Battery health percentage (if known)
    pub health: Option<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

/// Audio/volume status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStatus {
    /// Volume percentage (0-100)
    pub volume: u8,
    /// Whether muted
    pub muted: bool,
    /// Current output device name
    pub output_device: String,
    /// Whether microphone is muted
    pub mic_muted: bool,
}

/// Network connectivity indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIndicator {
    /// Connection type
    pub connection_type: NetworkType,
    /// Signal strength (0-100) for wireless
    pub signal_strength: Option<u8>,
    /// Network name (SSID for WiFi)
    pub network_name: Option<String>,
    /// Whether VPN is active
    pub vpn_active: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum NetworkType {
    None,
    Ethernet,
    Wifi,
    Cellular,
    VpnOnly,
}

/// Bluetooth status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothStatus {
    /// Whether Bluetooth is available
    pub available: bool,
    /// Whether Bluetooth is powered on
    pub powered: bool,
    /// Number of connected devices
    pub connected_devices: u8,
}

/// Date and time information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateTimeInfo {
    /// Current timestamp (Unix epoch)
    pub timestamp: i64,
    /// Timezone name
    pub timezone: String,
    /// Formatted time string
    pub time_formatted: String,
    /// Formatted date string
    pub date_formatted: String,
}

/// System resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage (0-100)
    pub cpu_percent: f32,
    /// Memory usage percentage (0-100)
    pub memory_percent: f32,
    /// Used memory in bytes
    pub memory_used: u64,
    /// Total memory in bytes
    pub memory_total: u64,
}

/// Applets backend
pub struct AppletsBackend;

impl AppletsBackend {
    /// Get all applet status in one call
    pub fn status() -> AppletStatus {
        AppletStatus {
            battery: Self::battery_status().ok(),
            audio: Self::audio_status().unwrap_or_else(|_| AudioStatus {
                volume: 50,
                muted: false,
                output_device: "Unknown".to_string(),
                mic_muted: false,
            }),
            network: Self::network_indicator().unwrap_or_else(|_| NetworkIndicator {
                connection_type: NetworkType::None,
                signal_strength: None,
                network_name: None,
                vpn_active: false,
            }),
            bluetooth: Self::bluetooth_status().unwrap_or_else(|_| BluetoothStatus {
                available: false,
                powered: false,
                connected_devices: 0,
            }),
            datetime: Self::datetime_info(),
            resources: Self::resource_usage().unwrap_or_else(|_| ResourceUsage {
                cpu_percent: 0.0,
                memory_percent: 0.0,
                memory_used: 0,
                memory_total: 0,
            }),
        }
    }

    /// Get battery status from /sys/class/power_supply
    pub fn battery_status() -> Result<BatteryStatus> {
        let power_supply = Path::new("/sys/class/power_supply");

        // Find the first battery
        let battery_path = fs::read_dir(power_supply)?
            .filter_map(|e| e.ok())
            .find(|e| {
                let type_path = e.path().join("type");
                fs::read_to_string(type_path)
                    .map(|t| t.trim() == "Battery")
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .context("No battery found")?;

        let read_file = |name: &str| -> Option<String> {
            fs::read_to_string(battery_path.join(name))
                .ok()
                .map(|s| s.trim().to_string())
        };

        let capacity = read_file("capacity")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let status = read_file("status").unwrap_or_default();
        let state = match status.as_str() {
            "Charging" => BatteryState::Charging,
            "Discharging" => BatteryState::Discharging,
            "Full" => BatteryState::Full,
            "Not charging" => BatteryState::NotCharging,
            _ => BatteryState::Unknown,
        };

        // Calculate time remaining
        let time_remaining = Self::calculate_battery_time(&battery_path);

        // Get power draw
        let power_draw = read_file("power_now")
            .and_then(|s| s.parse::<u64>().ok())
            .map(|p| p as f32 / 1_000_000.0); // Convert µW to W

        // Get health
        let health = read_file("charge_full")
            .and_then(|full| {
                read_file("charge_full_design").map(|design| (full, design))
            })
            .and_then(|(full, design)| {
                let full: u64 = full.parse().ok()?;
                let design: u64 = design.parse().ok()?;
                if design > 0 {
                    Some(((full * 100) / design) as u8)
                } else {
                    None
                }
            });

        Ok(BatteryStatus {
            percentage: capacity,
            state,
            time_remaining,
            power_draw,
            health,
        })
    }

    /// Calculate battery time remaining
    fn calculate_battery_time(battery_path: &Path) -> Option<u64> {
        let read_file = |name: &str| -> Option<u64> {
            fs::read_to_string(battery_path.join(name))
                .ok()
                .and_then(|s| s.trim().parse().ok())
        };

        let energy_now = read_file("energy_now")?;
        let power_now = read_file("power_now")?;

        if power_now == 0 {
            return None;
        }

        let status = fs::read_to_string(battery_path.join("status"))
            .ok()?
            .trim()
            .to_string();

        let hours = match status.as_str() {
            "Discharging" => energy_now as f64 / power_now as f64,
            "Charging" => {
                let energy_full = read_file("energy_full")?;
                (energy_full - energy_now) as f64 / power_now as f64
            }
            _ => return None,
        };

        Some((hours * 3600.0) as u64)
    }

    /// Get audio status using pactl/wpctl
    pub fn audio_status() -> Result<AudioStatus> {
        // Try wireplumber first, then pulseaudio

        // Try wpctl (wireplumber)
        if let Ok(status) = Self::audio_status_wpctl() {
            return Ok(status);
        }

        // Fallback to pactl (pulseaudio)
        Self::audio_status_pactl()
    }

    fn audio_status_wpctl() -> Result<AudioStatus> {
        let output = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .context("Failed to run wpctl")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Output format: "Volume: 0.50" or "Volume: 0.50 [MUTED]"

        let mut volume = 50u8;
        let mut muted = false;

        if let Some(vol_str) = stdout.strip_prefix("Volume: ") {
            let parts: Vec<&str> = vol_str.split_whitespace().collect();
            if let Some(vol) = parts.first() {
                if let Ok(v) = vol.parse::<f32>() {
                    volume = (v * 100.0).round() as u8;
                }
            }
            muted = stdout.contains("[MUTED]");
        }

        // Get current sink name
        let sink_output = Command::new("wpctl")
            .args(["inspect", "@DEFAULT_AUDIO_SINK@"])
            .output()?;
        let sink_stdout = String::from_utf8_lossy(&sink_output.stdout);
        let output_device = sink_stdout
            .lines()
            .find(|l| l.contains("node.description"))
            .and_then(|l| l.split('=').nth(1))
            .map(|s| s.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| "Default".to_string());

        // Check mic mute
        let mic_output = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
            .output()?;
        let mic_muted = String::from_utf8_lossy(&mic_output.stdout).contains("[MUTED]");

        Ok(AudioStatus {
            volume,
            muted,
            output_device,
            mic_muted,
        })
    }

    fn audio_status_pactl() -> Result<AudioStatus> {
        let output = Command::new("pactl")
            .args(["get-sink-volume", "@DEFAULT_SINK@"])
            .output()
            .context("Failed to run pactl")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let volume = stdout
            .split_whitespace()
            .find(|s| s.ends_with('%'))
            .and_then(|s| s.trim_end_matches('%').parse().ok())
            .unwrap_or(50);

        let mute_output = Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()?;
        let muted = String::from_utf8_lossy(&mute_output.stdout).contains("yes");

        // Get mic mute
        let mic_mute_output = Command::new("pactl")
            .args(["get-source-mute", "@DEFAULT_SOURCE@"])
            .output()?;
        let mic_muted = String::from_utf8_lossy(&mic_mute_output.stdout).contains("yes");

        Ok(AudioStatus {
            volume,
            muted,
            output_device: "Default".to_string(),
            mic_muted,
        })
    }

    /// Set audio volume
    pub fn set_volume(volume: u8) -> Result<()> {
        let vol_str = format!("{}%", volume.min(100));

        // Try wpctl first
        if Command::new("wpctl")
            .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &vol_str])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Ok(());
        }

        // Fallback to pactl
        Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &vol_str])
            .output()
            .context("Failed to set volume")?;

        Ok(())
    }

    /// Toggle mute
    pub fn toggle_mute() -> Result<()> {
        // Try wpctl first
        if Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Ok(());
        }

        // Fallback to pactl
        Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            .output()
            .context("Failed to toggle mute")?;

        Ok(())
    }

    /// Get network indicator status
    pub fn network_indicator() -> Result<NetworkIndicator> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "TYPE,STATE,CONNECTION", "device"])
            .output()
            .context("Failed to run nmcli")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut connection_type = NetworkType::None;
        let mut network_name = None;
        let mut signal_strength = None;

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 && parts[1] == "connected" {
                let dev_type = parts[0];
                let conn_name = parts[2];

                match dev_type {
                    "ethernet" => {
                        connection_type = NetworkType::Ethernet;
                        network_name = Some(conn_name.to_string());
                    }
                    "wifi" => {
                        connection_type = NetworkType::Wifi;
                        network_name = Some(conn_name.to_string());
                        signal_strength = Self::get_wifi_signal();
                    }
                    "gsm" | "cdma" => {
                        if connection_type == NetworkType::None {
                            connection_type = NetworkType::Cellular;
                            network_name = Some(conn_name.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check VPN
        let vpn_output = Command::new("nmcli")
            .args(["-t", "-f", "TYPE,STATE", "connection", "show", "--active"])
            .output()?;
        let vpn_active = String::from_utf8_lossy(&vpn_output.stdout)
            .lines()
            .any(|l| l.starts_with("vpn:") && l.contains("activated"));

        if vpn_active && connection_type == NetworkType::None {
            connection_type = NetworkType::VpnOnly;
        }

        Ok(NetworkIndicator {
            connection_type,
            signal_strength,
            network_name,
            vpn_active,
        })
    }

    /// Get WiFi signal strength
    fn get_wifi_signal() -> Option<u8> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "IN-USE,SIGNAL", "device", "wifi", "list"])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("*:") {
                return line
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.parse().ok());
            }
        }
        None
    }

    /// Get Bluetooth status
    pub fn bluetooth_status() -> Result<BluetoothStatus> {
        let output = Command::new("bluetoothctl")
            .arg("show")
            .output()
            .context("Failed to run bluetoothctl")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let powered = stdout
            .lines()
            .any(|l| l.contains("Powered:") && l.contains("yes"));

        // Count connected devices
        let devices_output = Command::new("bluetoothctl")
            .arg("devices")
            .arg("Connected")
            .output()?;
        let connected_devices = String::from_utf8_lossy(&devices_output.stdout)
            .lines()
            .count() as u8;

        Ok(BluetoothStatus {
            available: true,
            powered,
            connected_devices,
        })
    }

    /// Toggle Bluetooth power
    pub fn toggle_bluetooth() -> Result<()> {
        let status = Self::bluetooth_status()?;
        let action = if status.powered { "off" } else { "on" };

        Command::new("bluetoothctl")
            .args(["power", action])
            .output()
            .context("Failed to toggle Bluetooth")?;

        Ok(())
    }

    /// Get date/time information
    pub fn datetime_info() -> DateTimeInfo {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let timezone = std::env::var("TZ")
            .or_else(|_| {
                fs::read_link("/etc/localtime")
                    .ok()
                    .and_then(|p| {
                        p.to_string_lossy()
                            .split("/zoneinfo/")
                            .nth(1)
                            .map(|s| s.to_string())
                    })
                    .ok_or(std::env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| "UTC".to_string());

        // Get formatted time using date command for proper locale support
        let time_formatted = Command::new("date")
            .arg("+%H:%M")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "00:00".to_string());

        let date_formatted = Command::new("date")
            .arg("+%a, %b %d")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());

        DateTimeInfo {
            timestamp,
            timezone,
            time_formatted,
            date_formatted,
        }
    }

    /// Get system resource usage
    pub fn resource_usage() -> Result<ResourceUsage> {
        // Read /proc/meminfo for memory
        let meminfo = fs::read_to_string("/proc/meminfo")?;

        let mut mem_total = 0u64;
        let mut mem_available = 0u64;

        for line in meminfo.lines() {
            if let Some(value) = line.strip_prefix("MemTotal:") {
                mem_total = value
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
                    * 1024; // Convert KB to bytes
            } else if let Some(value) = line.strip_prefix("MemAvailable:") {
                mem_available = value
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0)
                    * 1024;
            }
        }

        let mem_used = mem_total.saturating_sub(mem_available);
        let memory_percent = if mem_total > 0 {
            (mem_used as f32 / mem_total as f32) * 100.0
        } else {
            0.0
        };

        // Read /proc/stat for CPU
        let cpu_percent = Self::calculate_cpu_usage().unwrap_or(0.0);

        Ok(ResourceUsage {
            cpu_percent,
            memory_percent,
            memory_used: mem_used,
            memory_total: mem_total,
        })
    }

    /// Calculate CPU usage (simple single-sample method)
    fn calculate_cpu_usage() -> Result<f32> {
        let read_cpu_stats = || -> Result<(u64, u64)> {
            let stat = fs::read_to_string("/proc/stat")?;
            let cpu_line = stat.lines().next().context("No CPU line")?;
            let values: Vec<u64> = cpu_line
                .split_whitespace()
                .skip(1) // Skip "cpu" prefix
                .filter_map(|s| s.parse().ok())
                .collect();

            if values.len() < 4 {
                anyhow::bail!("Invalid CPU stats");
            }

            let idle = values[3];
            let total: u64 = values.iter().sum();
            Ok((idle, total))
        };

        let (idle1, total1) = read_cpu_stats()?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        let (idle2, total2) = read_cpu_stats()?;

        let idle_delta = idle2.saturating_sub(idle1);
        let total_delta = total2.saturating_sub(total1);

        if total_delta == 0 {
            return Ok(0.0);
        }

        let usage = 100.0 * (1.0 - (idle_delta as f32 / total_delta as f32));
        Ok(usage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime() {
        let dt = AppletsBackend::datetime_info();
        assert!(!dt.time_formatted.is_empty());
    }
}
