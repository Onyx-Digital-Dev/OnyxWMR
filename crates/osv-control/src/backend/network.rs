//! Network management backend using nmcli
//!
//! Provides WiFi scanning, connection management, and VPN support
//! including OpenVPN through NetworkManager.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// WiFi access point information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    /// SSID of the network
    pub ssid: String,
    /// Signal strength (0-100)
    pub signal: u8,
    /// Security type (e.g., "WPA2", "WPA3", "Open")
    pub security: String,
    /// Whether currently connected
    pub connected: bool,
    /// BSSID (MAC address)
    pub bssid: String,
    /// Frequency in MHz
    pub frequency: u32,
}

/// VPN connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConnection {
    /// Connection name
    pub name: String,
    /// VPN type (e.g., "openvpn", "wireguard")
    pub vpn_type: String,
    /// Whether currently active
    pub active: bool,
    /// UUID for the connection
    pub uuid: String,
}

/// Network connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    /// Whether WiFi is enabled
    pub wifi_enabled: bool,
    /// Whether networking is enabled
    pub networking_enabled: bool,
    /// Currently connected WiFi SSID (if any)
    pub connected_wifi: Option<String>,
    /// Currently active VPN (if any)
    pub active_vpn: Option<String>,
    /// Primary connection type
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionType {
    None,
    Ethernet,
    Wifi,
    Cellular,
}

/// Network manager backend
pub struct NetworkBackend;

impl NetworkBackend {
    /// Get current network status
    pub fn status() -> Result<NetworkStatus> {
        let wifi_enabled = Self::is_wifi_enabled()?;
        let networking_enabled = Self::is_networking_enabled()?;

        // Get active connections
        let output = Command::new("nmcli")
            .args(["-t", "-f", "NAME,TYPE,DEVICE", "connection", "show", "--active"])
            .output()
            .context("Failed to run nmcli")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut connected_wifi = None;
        let mut active_vpn = None;
        let mut connection_type = ConnectionType::None;

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let conn_type = parts[1];

                match conn_type {
                    "802-11-wireless" => {
                        connected_wifi = Some(name.to_string());
                        connection_type = ConnectionType::Wifi;
                    }
                    "802-3-ethernet" => {
                        if connection_type == ConnectionType::None {
                            connection_type = ConnectionType::Ethernet;
                        }
                    }
                    "vpn" => {
                        active_vpn = Some(name.to_string());
                    }
                    "gsm" | "cdma" => {
                        if connection_type == ConnectionType::None {
                            connection_type = ConnectionType::Cellular;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(NetworkStatus {
            wifi_enabled,
            networking_enabled,
            connected_wifi,
            active_vpn,
            connection_type,
        })
    }

    /// Check if WiFi hardware is enabled
    pub fn is_wifi_enabled() -> Result<bool> {
        let output = Command::new("nmcli")
            .args(["radio", "wifi"])
            .output()
            .context("Failed to check WiFi status")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim() == "enabled")
    }

    /// Check if networking is enabled
    pub fn is_networking_enabled() -> Result<bool> {
        let output = Command::new("nmcli")
            .args(["networking"])
            .output()
            .context("Failed to check networking status")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim() == "enabled")
    }

    /// Enable or disable WiFi
    pub fn set_wifi_enabled(enabled: bool) -> Result<()> {
        let state = if enabled { "on" } else { "off" };
        let output = Command::new("nmcli")
            .args(["radio", "wifi", state])
            .output()
            .context("Failed to set WiFi state")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to {} WiFi: {}",
                if enabled { "enable" } else { "disable" },
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    /// Scan for available WiFi networks
    pub fn scan_wifi() -> Result<Vec<WifiNetwork>> {
        // Trigger a rescan
        let _ = Command::new("nmcli")
            .args(["device", "wifi", "rescan"])
            .output();

        // Small delay to allow scan to complete
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Get list of networks
        let output = Command::new("nmcli")
            .args([
                "-t",
                "-f",
                "SSID,SIGNAL,SECURITY,IN-USE,BSSID,FREQ",
                "device",
                "wifi",
                "list",
            ])
            .output()
            .context("Failed to list WiFi networks")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut networks = Vec::new();
        let mut seen_ssids = std::collections::HashSet::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 6 {
                let ssid = parts[0].to_string();

                // Skip empty SSIDs and duplicates (keep strongest signal)
                if ssid.is_empty() || seen_ssids.contains(&ssid) {
                    continue;
                }
                seen_ssids.insert(ssid.clone());

                let signal = parts[1].parse().unwrap_or(0);
                let security = parts[2].to_string();
                let connected = parts[3] == "*";
                let bssid = parts[4].to_string();
                let frequency = parts[5]
                    .trim_end_matches(" MHz")
                    .parse()
                    .unwrap_or(0);

                networks.push(WifiNetwork {
                    ssid,
                    signal,
                    security,
                    connected,
                    bssid,
                    frequency,
                });
            }
        }

        // Sort by signal strength (strongest first)
        networks.sort_by(|a, b| b.signal.cmp(&a.signal));

        Ok(networks)
    }

    /// Connect to a WiFi network
    pub fn connect_wifi(ssid: &str, password: Option<&str>) -> Result<()> {
        let mut args = vec!["device", "wifi", "connect", ssid];

        if let Some(pwd) = password {
            args.push("password");
            args.push(pwd);
        }

        let output = Command::new("nmcli")
            .args(&args)
            .output()
            .context("Failed to connect to WiFi")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to connect to '{}': {}", ssid, stderr);
        }

        Ok(())
    }

    /// Disconnect from current WiFi network
    pub fn disconnect_wifi() -> Result<()> {
        // Find the WiFi device
        let output = Command::new("nmcli")
            .args(["-t", "-f", "DEVICE,TYPE", "device"])
            .output()
            .context("Failed to list devices")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let wifi_device = stdout
            .lines()
            .find(|line| line.contains(":wifi"))
            .and_then(|line| line.split(':').next())
            .context("No WiFi device found")?;

        let output = Command::new("nmcli")
            .args(["device", "disconnect", wifi_device])
            .output()
            .context("Failed to disconnect")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to disconnect: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Forget a saved WiFi network
    pub fn forget_wifi(ssid: &str) -> Result<()> {
        let output = Command::new("nmcli")
            .args(["connection", "delete", ssid])
            .output()
            .context("Failed to forget network")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to forget '{}': {}",
                ssid,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// List all VPN connections
    pub fn list_vpns() -> Result<Vec<VpnConnection>> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "NAME,TYPE,UUID", "connection", "show"])
            .output()
            .context("Failed to list connections")?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Get active VPN
        let active_output = Command::new("nmcli")
            .args([
                "-t",
                "-f",
                "NAME,TYPE",
                "connection",
                "show",
                "--active",
            ])
            .output()?;
        let active_stdout = String::from_utf8_lossy(&active_output.stdout);
        let active_vpns: std::collections::HashSet<_> = active_stdout
            .lines()
            .filter(|line| line.contains(":vpn"))
            .filter_map(|line| line.split(':').next())
            .collect();

        let mut vpns = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 && parts[1] == "vpn" {
                let name = parts[0].to_string();
                let uuid = parts[2].to_string();

                // Get VPN type from connection details
                let vpn_type = Self::get_vpn_type(&uuid).unwrap_or_else(|_| "unknown".to_string());

                vpns.push(VpnConnection {
                    name: name.clone(),
                    vpn_type,
                    active: active_vpns.contains(name.as_str()),
                    uuid,
                });
            }
        }

        Ok(vpns)
    }

    /// Get VPN type from connection UUID
    fn get_vpn_type(uuid: &str) -> Result<String> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "vpn.service-type", "connection", "show", uuid])
            .output()
            .context("Failed to get VPN type")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let vpn_type = stdout
            .lines()
            .next()
            .and_then(|line| line.split(':').nth(1))
            .map(|s| {
                // Extract type from service name like "org.freedesktop.NetworkManager.openvpn"
                s.rsplit('.').next().unwrap_or(s).to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());

        Ok(vpn_type)
    }

    /// Connect to a VPN
    pub fn connect_vpn(name: &str) -> Result<()> {
        let output = Command::new("nmcli")
            .args(["connection", "up", name])
            .output()
            .context("Failed to connect to VPN")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to connect to VPN '{}': {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Disconnect from a VPN
    pub fn disconnect_vpn(name: &str) -> Result<()> {
        let output = Command::new("nmcli")
            .args(["connection", "down", name])
            .output()
            .context("Failed to disconnect VPN")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to disconnect VPN '{}': {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Import an OpenVPN configuration file
    pub fn import_openvpn(config_path: &Path, connection_name: Option<&str>) -> Result<String> {
        if !config_path.exists() {
            anyhow::bail!("OpenVPN config file not found: {}", config_path.display());
        }

        let output = Command::new("nmcli")
            .args([
                "connection",
                "import",
                "type",
                "openvpn",
                "file",
                config_path.to_str().context("Invalid path")?,
            ])
            .output()
            .context("Failed to import OpenVPN config")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to import OpenVPN config: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract connection name from output
        // Output is like: "Connection 'vpn-name' (uuid) successfully added."
        let imported_name = stdout
            .split('\'')
            .nth(1)
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                config_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("imported-vpn")
                    .to_string()
            });

        // Rename if a custom name was provided
        if let Some(new_name) = connection_name {
            if new_name != imported_name {
                let _ = Command::new("nmcli")
                    .args([
                        "connection",
                        "modify",
                        &imported_name,
                        "connection.id",
                        new_name,
                    ])
                    .output();
                return Ok(new_name.to_string());
            }
        }

        Ok(imported_name)
    }

    /// Delete a VPN connection
    pub fn delete_vpn(name: &str) -> Result<()> {
        let output = Command::new("nmcli")
            .args(["connection", "delete", name])
            .output()
            .context("Failed to delete VPN")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to delete VPN '{}': {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Import a WireGuard configuration file
    pub fn import_wireguard(config_path: &Path, connection_name: Option<&str>) -> Result<String> {
        if !config_path.exists() {
            anyhow::bail!(
                "WireGuard config file not found: {}",
                config_path.display()
            );
        }

        let output = Command::new("nmcli")
            .args([
                "connection",
                "import",
                "type",
                "wireguard",
                "file",
                config_path.to_str().context("Invalid path")?,
            ])
            .output()
            .context("Failed to import WireGuard config")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to import WireGuard config: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let imported_name = stdout
            .split('\'')
            .nth(1)
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                config_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("imported-wg")
                    .to_string()
            });

        if let Some(new_name) = connection_name {
            if new_name != imported_name {
                let _ = Command::new("nmcli")
                    .args([
                        "connection",
                        "modify",
                        &imported_name,
                        "connection.id",
                        new_name,
                    ])
                    .output();
                return Ok(new_name.to_string());
            }
        }

        Ok(imported_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_parsing() {
        // This would need a mock or integration test environment
    }
}
