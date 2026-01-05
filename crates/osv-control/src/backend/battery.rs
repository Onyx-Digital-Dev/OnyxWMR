//! Battery monitoring backend
//!
//! Provides battery status, charge level, health, and history tracking.
//! Reads from /sys/class/power_supply/ and optionally UPower for history.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POWER_SUPPLY_PATH: &str = "/sys/class/power_supply";

/// Battery charging state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryState {
    /// Battery is charging
    Charging,
    /// Battery is discharging (on battery power)
    Discharging,
    /// Battery is full and on AC power
    Full,
    /// Battery is not charging (plugged in but not charging, e.g., threshold reached)
    NotCharging,
    /// Unknown state
    Unknown,
}

impl From<&str> for BatteryState {
    fn from(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "charging" => BatteryState::Charging,
            "discharging" => BatteryState::Discharging,
            "full" => BatteryState::Full,
            "not charging" => BatteryState::NotCharging,
            _ => BatteryState::Unknown,
        }
    }
}

/// Battery health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryHealth {
    /// Battery is in good condition
    Good,
    /// Battery is degraded but functional
    Degraded,
    /// Battery needs replacement
    Critical,
    /// Health unknown
    Unknown,
}

/// Battery technology type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryTechnology {
    LithiumIon,
    LithiumPolymer,
    NickelMetalHydride,
    LeadAcid,
    Other(String),
    Unknown,
}

impl From<&str> for BatteryTechnology {
    fn from(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "li-ion" | "lion" | "lithium-ion" => BatteryTechnology::LithiumIon,
            "li-poly" | "lipo" | "lithium-polymer" => BatteryTechnology::LithiumPolymer,
            "nimh" | "ni-mh" => BatteryTechnology::NickelMetalHydride,
            "lead-acid" | "pbac" => BatteryTechnology::LeadAcid,
            "" | "unknown" => BatteryTechnology::Unknown,
            other => BatteryTechnology::Other(other.to_string()),
        }
    }
}

/// Information about a single battery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    /// Battery name/identifier (e.g., "BAT0")
    pub name: String,
    /// Current charge percentage (0-100)
    pub percentage: u8,
    /// Current charging state
    pub state: BatteryState,
    /// Time to empty when discharging (in seconds), if available
    pub time_to_empty: Option<u64>,
    /// Time to full when charging (in seconds), if available
    pub time_to_full: Option<u64>,
    /// Current energy in microWatt-hours
    pub energy_now: Option<u64>,
    /// Full charge capacity in microWatt-hours
    pub energy_full: Option<u64>,
    /// Design capacity in microWatt-hours
    pub energy_full_design: Option<u64>,
    /// Current voltage in microVolts
    pub voltage_now: Option<u64>,
    /// Current power draw/charge rate in microWatts
    pub power_now: Option<u64>,
    /// Battery health estimate
    pub health: BatteryHealth,
    /// Health percentage (current capacity vs design capacity)
    pub health_percentage: Option<u8>,
    /// Battery technology
    pub technology: BatteryTechnology,
    /// Manufacturer
    pub manufacturer: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// Serial number
    pub serial: Option<String>,
    /// Cycle count if available
    pub cycle_count: Option<u32>,
    /// Temperature in millidegrees Celsius
    pub temperature: Option<i32>,
}

/// AC adapter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcAdapterInfo {
    /// Adapter name (e.g., "AC", "ADP1")
    pub name: String,
    /// Whether AC is connected
    pub online: bool,
}

/// Complete power supply status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSupplyStatus {
    /// All detected batteries
    pub batteries: Vec<BatteryInfo>,
    /// All detected AC adapters
    pub ac_adapters: Vec<AcAdapterInfo>,
    /// Whether the system is on AC power
    pub on_ac_power: bool,
    /// Combined battery percentage (weighted average)
    pub combined_percentage: Option<u8>,
    /// Overall charging state
    pub overall_state: BatteryState,
}

/// Historical battery data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryHistoryPoint {
    /// Unix timestamp
    pub timestamp: u64,
    /// Charge percentage
    pub percentage: u8,
    /// Charging state
    pub state: BatteryState,
    /// Power draw in watts (negative = charging)
    pub power_watts: Option<f64>,
}

/// Battery backend with optional history tracking
pub struct BatteryBackend {
    /// History of battery readings
    history: Vec<BatteryHistoryPoint>,
    /// Maximum history entries to keep
    max_history: usize,
    /// Last update time
    last_update: Option<Instant>,
}

impl Default for BatteryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl BatteryBackend {
    /// Create a new battery backend
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 1000, // ~16 hours at 1 reading/minute
            last_update: None,
        }
    }

    /// Get current power supply status
    pub fn get_status(&mut self) -> Result<PowerSupplyStatus> {
        let mut batteries = Vec::new();
        let mut ac_adapters = Vec::new();

        let power_supply_dir = Path::new(POWER_SUPPLY_PATH);
        if !power_supply_dir.exists() {
            return Ok(PowerSupplyStatus {
                batteries: vec![],
                ac_adapters: vec![],
                on_ac_power: true, // Assume AC if no battery info
                combined_percentage: None,
                overall_state: BatteryState::Unknown,
            });
        }

        for entry in fs::read_dir(power_supply_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            let supply_type = read_sysfs_string(&path.join("type")).unwrap_or_default();

            match supply_type.trim().to_lowercase().as_str() {
                "battery" => {
                    if let Ok(battery) = Self::read_battery(&path, &name) {
                        batteries.push(battery);
                    }
                }
                "mains" | "usb" => {
                    if let Ok(adapter) = Self::read_ac_adapter(&path, &name) {
                        ac_adapters.push(adapter);
                    }
                }
                _ => {}
            }
        }

        // Determine if on AC power
        let on_ac_power = ac_adapters.iter().any(|a| a.online) || batteries.is_empty();

        // Calculate combined percentage
        let combined_percentage = if batteries.is_empty() {
            None
        } else {
            let total: u32 = batteries.iter().map(|b| b.percentage as u32).sum();
            Some((total / batteries.len() as u32) as u8)
        };

        // Determine overall state
        let overall_state = if batteries.is_empty() {
            BatteryState::Unknown
        } else if batteries.iter().any(|b| b.state == BatteryState::Charging) {
            BatteryState::Charging
        } else if batteries.iter().all(|b| b.state == BatteryState::Full) {
            BatteryState::Full
        } else if batteries.iter().any(|b| b.state == BatteryState::Discharging) {
            BatteryState::Discharging
        } else {
            BatteryState::NotCharging
        };

        // Record history point
        if let Some(pct) = combined_percentage {
            let power_watts = batteries.first().and_then(|b| {
                b.power_now.map(|p| p as f64 / 1_000_000.0)
            });

            self.record_history(pct, overall_state, power_watts);
        }

        Ok(PowerSupplyStatus {
            batteries,
            ac_adapters,
            on_ac_power,
            combined_percentage,
            overall_state,
        })
    }

    /// Read battery information from sysfs
    fn read_battery(path: &Path, name: &str) -> Result<BatteryInfo> {
        // Read charge/energy values (different batteries use different units)
        let (energy_now, energy_full, energy_full_design) = Self::read_energy_values(path);

        // Calculate percentage
        let percentage = if let Some(cap) = read_sysfs_u64(&path.join("capacity")) {
            cap.min(100) as u8
        } else if let (Some(now), Some(full)) = (energy_now, energy_full) {
            if full > 0 {
                ((now * 100) / full).min(100) as u8
            } else {
                0
            }
        } else {
            0
        };

        // Read state
        let state = read_sysfs_string(&path.join("status"))
            .map(|s| BatteryState::from(s.as_str()))
            .unwrap_or(BatteryState::Unknown);

        // Calculate health
        let (health, health_percentage) = if let (Some(full), Some(design)) =
            (energy_full, energy_full_design)
        {
            if design > 0 {
                let pct = ((full * 100) / design).min(100) as u8;
                let health = if pct >= 80 {
                    BatteryHealth::Good
                } else if pct >= 50 {
                    BatteryHealth::Degraded
                } else {
                    BatteryHealth::Critical
                };
                (health, Some(pct))
            } else {
                (BatteryHealth::Unknown, None)
            }
        } else {
            (BatteryHealth::Unknown, None)
        };

        // Read power draw
        let power_now = read_sysfs_u64(&path.join("power_now"))
            .or_else(|| read_sysfs_u64(&path.join("current_now")));

        // Read voltage
        let voltage_now = read_sysfs_u64(&path.join("voltage_now"));

        // Calculate time estimates
        let (time_to_empty, time_to_full) = Self::calculate_time_estimates(
            energy_now,
            energy_full,
            power_now,
            state,
        );

        Ok(BatteryInfo {
            name: name.to_string(),
            percentage,
            state,
            time_to_empty,
            time_to_full,
            energy_now,
            energy_full,
            energy_full_design,
            voltage_now,
            power_now,
            health,
            health_percentage,
            technology: read_sysfs_string(&path.join("technology"))
                .map(|s| BatteryTechnology::from(s.as_str()))
                .unwrap_or(BatteryTechnology::Unknown),
            manufacturer: read_sysfs_string(&path.join("manufacturer")),
            model: read_sysfs_string(&path.join("model_name")),
            serial: read_sysfs_string(&path.join("serial_number")),
            cycle_count: read_sysfs_u64(&path.join("cycle_count")).map(|c| c as u32),
            temperature: read_sysfs_i64(&path.join("temp")).map(|t| t as i32),
        })
    }

    /// Read energy values, handling both energy_* and charge_* naming conventions
    fn read_energy_values(path: &Path) -> (Option<u64>, Option<u64>, Option<u64>) {
        // Try energy_* first (in µWh)
        let energy_now = read_sysfs_u64(&path.join("energy_now"));
        let energy_full = read_sysfs_u64(&path.join("energy_full"));
        let energy_full_design = read_sysfs_u64(&path.join("energy_full_design"));

        if energy_now.is_some() || energy_full.is_some() {
            return (energy_now, energy_full, energy_full_design);
        }

        // Fallback to charge_* (in µAh) - need voltage to convert
        let voltage = read_sysfs_u64(&path.join("voltage_now")).unwrap_or(3_700_000); // Default 3.7V

        let charge_now = read_sysfs_u64(&path.join("charge_now"));
        let charge_full = read_sysfs_u64(&path.join("charge_full"));
        let charge_full_design = read_sysfs_u64(&path.join("charge_full_design"));

        // Convert µAh to µWh: µWh = µAh * V = µAh * (µV / 1_000_000)
        let convert = |charge: Option<u64>| -> Option<u64> {
            charge.map(|c| (c * voltage) / 1_000_000)
        };

        (convert(charge_now), convert(charge_full), convert(charge_full_design))
    }

    /// Calculate time to empty/full estimates
    fn calculate_time_estimates(
        energy_now: Option<u64>,
        energy_full: Option<u64>,
        power_now: Option<u64>,
        state: BatteryState,
    ) -> (Option<u64>, Option<u64>) {
        let power = match power_now {
            Some(p) if p > 0 => p,
            _ => return (None, None),
        };

        match state {
            BatteryState::Discharging => {
                if let Some(now) = energy_now {
                    // Time in seconds = (energy in µWh) / (power in µW) * 3600
                    let seconds = (now * 3600) / power;
                    return (Some(seconds), None);
                }
            }
            BatteryState::Charging => {
                if let (Some(now), Some(full)) = (energy_now, energy_full) {
                    if full > now {
                        let remaining = full - now;
                        let seconds = (remaining * 3600) / power;
                        return (None, Some(seconds));
                    }
                }
            }
            _ => {}
        }

        (None, None)
    }

    /// Read AC adapter information
    fn read_ac_adapter(path: &Path, name: &str) -> Result<AcAdapterInfo> {
        let online = read_sysfs_u64(&path.join("online")).unwrap_or(0) == 1;

        Ok(AcAdapterInfo {
            name: name.to_string(),
            online,
        })
    }

    /// Record a history point
    fn record_history(&mut self, percentage: u8, state: BatteryState, power_watts: Option<f64>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Only record if enough time has passed (at least 30 seconds)
        if let Some(last) = self.history.last() {
            if now - last.timestamp < 30 {
                return;
            }
        }

        self.history.push(BatteryHistoryPoint {
            timestamp: now,
            percentage,
            state,
            power_watts,
        });

        // Trim old entries
        while self.history.len() > self.max_history {
            self.history.remove(0);
        }

        self.last_update = Some(Instant::now());
    }

    /// Get battery history for the last N minutes
    pub fn get_history(&self, minutes: u32) -> Vec<BatteryHistoryPoint> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub((minutes as u64) * 60);

        self.history
            .iter()
            .filter(|p| p.timestamp >= cutoff)
            .cloned()
            .collect()
    }

    /// Get UPower history if available (for longer-term history)
    pub fn get_upower_history(battery: &str, hours: u32) -> Result<Vec<BatteryHistoryPoint>> {
        use std::process::Command;

        let output = Command::new("upower")
            .args(["-i", &format!("/org/freedesktop/UPower/devices/battery_{}", battery)])
            .output()
            .context("Failed to run upower")?;

        // UPower history parsing would go here
        // For now, return empty - full implementation would parse upower dump-history

        Ok(vec![])
    }

    /// Get estimated battery life based on recent usage
    pub fn estimate_battery_life(&self) -> Option<Duration> {
        // Need at least 5 minutes of history
        let history = self.get_history(10);
        if history.len() < 2 {
            return None;
        }

        // Only estimate during discharge
        let discharging: Vec<_> = history
            .iter()
            .filter(|p| p.state == BatteryState::Discharging)
            .collect();

        if discharging.len() < 2 {
            return None;
        }

        let first = discharging.first()?;
        let last = discharging.last()?;

        let time_delta = last.timestamp.saturating_sub(first.timestamp);
        let pct_delta = first.percentage.saturating_sub(last.percentage);

        if pct_delta == 0 || time_delta == 0 {
            return None;
        }

        // Extrapolate: if we lost pct_delta% in time_delta seconds,
        // how long until 0%?
        let seconds_per_percent = time_delta / (pct_delta as u64);
        let remaining_seconds = (last.percentage as u64) * seconds_per_percent;

        Some(Duration::from_secs(remaining_seconds))
    }

    /// Check if battery is low (below threshold)
    pub fn is_low(&self, threshold: u8) -> bool {
        if let Ok(status) = Self::get_status_static() {
            if let Some(pct) = status.combined_percentage {
                return pct < threshold && status.overall_state == BatteryState::Discharging;
            }
        }
        false
    }

    /// Check if battery is critical (below critical threshold)
    pub fn is_critical(&self, threshold: u8) -> bool {
        if let Ok(status) = Self::get_status_static() {
            if let Some(pct) = status.combined_percentage {
                return pct < threshold && status.overall_state == BatteryState::Discharging;
            }
        }
        false
    }

    /// Static method to get status without history tracking
    pub fn get_status_static() -> Result<PowerSupplyStatus> {
        let mut backend = Self::new();
        backend.get_status()
    }
}

/// Read a string value from a sysfs file
fn read_sysfs_string(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read a u64 value from a sysfs file
fn read_sysfs_u64(path: &Path) -> Option<u64> {
    read_sysfs_string(path)?.parse().ok()
}

/// Read an i64 value from a sysfs file
fn read_sysfs_i64(path: &Path) -> Option<i64> {
    read_sysfs_string(path)?.parse().ok()
}

/// Format duration as human-readable string
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Format time remaining for display
pub fn format_time_remaining(seconds: u64) -> String {
    format_duration(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_state_from_str() {
        assert_eq!(BatteryState::from("Charging"), BatteryState::Charging);
        assert_eq!(BatteryState::from("Discharging"), BatteryState::Discharging);
        assert_eq!(BatteryState::from("Full"), BatteryState::Full);
        assert_eq!(BatteryState::from("Not charging"), BatteryState::NotCharging);
        assert_eq!(BatteryState::from("unknown"), BatteryState::Unknown);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m");
        assert_eq!(format_duration(Duration::from_secs(5400)), "1h 30m");
        assert_eq!(format_duration(Duration::from_secs(1800)), "30m");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m");
    }

    #[test]
    fn test_health_calculation() {
        // 90% of design = Good
        // 60% of design = Degraded
        // 40% of design = Critical
    }
}
