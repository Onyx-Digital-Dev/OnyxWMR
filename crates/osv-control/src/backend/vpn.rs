//! VPN Monitoring Backend
//!
//! Reliable VPN state and routing analysis. No guesswork.
//!
//! Guarantees:
//! - Accurate VPN interface detection (tun/tap/wg/ppp)
//! - Correct routing table parsing with VPN route identification
//! - WireGuard peer stats when available
//! - Per-interface bandwidth attribution
//! - Split tunnel detection
//!
//! Limitations (documented, not hidden):
//! - Per-application traffic flow requires eBPF (not implemented)
//! - OpenVPN internal state requires --status file configuration
//! - UDP connection tracking is best-effort

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::time::Instant;

// Interface Classification

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceType {
    Physical,
    WireGuard,
    OpenVPN,
    PPP,
    Bridge,
    Virtual,
    Loopback,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub iface_type: InterfaceType,
    pub is_vpn: bool,
    pub is_up: bool,
    pub mtu: u32,
    pub ipv4_addrs: Vec<String>,
    pub ipv6_addrs: Vec<String>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

// Routing

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub destination: String,
    pub gateway: Option<String>,
    pub interface: String,
    pub metric: u32,
    pub scope: RouteScope,
    pub protocol: RouteProtocol,
    pub is_default: bool,
    pub is_vpn_route: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteScope {
    Universe,
    Link,
    Host,
    Nowhere,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteProtocol {
    Kernel,
    Boot,
    Static,
    Dhcp,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingTable {
    pub default_interface: Option<String>,
    pub default_via_vpn: bool,
    pub routes: Vec<Route>,
    pub vpn_routes: Vec<Route>,
    pub split_tunnel_detected: bool,
    pub split_tunnel_subnets: Vec<String>,
}

// WireGuard

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardInterface {
    pub name: String,
    pub public_key: String,
    pub listen_port: Option<u16>,
    pub fwmark: Option<u32>,
    pub peers: Vec<WireGuardPeer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardPeer {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub allowed_ips: Vec<String>,
    pub latest_handshake: Option<u64>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub persistent_keepalive: Option<u16>,
}

// VPN Connection State

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VpnState {
    Connected,
    Connecting,
    Disconnected,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConnection {
    pub name: String,
    pub vpn_type: VpnType,
    pub state: VpnState,
    pub interface: Option<String>,
    pub server: Option<String>,
    pub local_ip: Option<String>,
    pub connected_since: Option<u64>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VpnType {
    WireGuard,
    OpenVPN,
    IPSec,
    PPTP,
    L2TP,
    SSTP,
    Unknown,
}

// Active Connections

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionProtocol {
    Tcp,
    Udp,
    Tcp6,
    Udp6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Established,
    SynSent,
    SynRecv,
    FinWait1,
    FinWait2,
    TimeWait,
    Close,
    CloseWait,
    LastAck,
    Listen,
    Closing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveConnection {
    pub protocol: ConnectionProtocol,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: ConnectionState,
    pub inode: u64,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub interface_hint: Option<String>,
    pub via_vpn: Option<bool>,
}

// Bandwidth Stats

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthSplit {
    pub vpn_rx_bytes: u64,
    pub vpn_tx_bytes: u64,
    pub direct_rx_bytes: u64,
    pub direct_tx_bytes: u64,
    pub vpn_rx_rate: f64,
    pub vpn_tx_rate: f64,
    pub direct_rx_rate: f64,
    pub direct_tx_rate: f64,
    pub vpn_percentage: f32,
}

// Full VPN Snapshot

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnSnapshot {
    pub timestamp: u64,
    pub interfaces: Vec<NetworkInterface>,
    pub vpn_interfaces: Vec<String>,
    pub routing: RoutingTable,
    pub wireguard: Vec<WireGuardInterface>,
    pub vpn_connections: Vec<VpnConnection>,
    pub bandwidth: BandwidthSplit,
    pub active_connections: Vec<ActiveConnection>,
    pub dns_servers: Vec<String>,
    pub dns_via_vpn: bool,
}

// Backend Implementation

pub struct VpnBackend {
    prev_stats: Option<(Instant, HashMap<String, (u64, u64)>)>,
    inode_to_pid: HashMap<u64, (u32, String)>,
}

impl Default for VpnBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl VpnBackend {
    pub fn new() -> Self {
        Self {
            prev_stats: None,
            inode_to_pid: HashMap::new(),
        }
    }

    pub fn snapshot(&mut self) -> Result<VpnSnapshot> {
        self.refresh_inode_map();

        let interfaces = self.get_interfaces()?;
        let vpn_interfaces: Vec<String> = interfaces
            .iter()
            .filter(|i| i.is_vpn)
            .map(|i| i.name.clone())
            .collect();

        let routing = self.get_routing_table(&vpn_interfaces)?;
        let wireguard = self.get_wireguard_interfaces();
        let vpn_connections = self.get_vpn_connections(&interfaces);
        let bandwidth = self.calculate_bandwidth_split(&interfaces);
        let active_connections = self.get_active_connections(&routing)?;
        let (dns_servers, dns_via_vpn) = self.get_dns_state(&vpn_interfaces);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(VpnSnapshot {
            timestamp,
            interfaces,
            vpn_interfaces,
            routing,
            wireguard,
            vpn_connections,
            bandwidth,
            active_connections,
            dns_servers,
            dns_via_vpn,
        })
    }

    // Interface Detection

    fn get_interfaces(&self) -> Result<Vec<NetworkInterface>> {
        let mut interfaces = Vec::new();
        let net_path = Path::new("/sys/class/net");

        let net_dev = fs::read_to_string("/proc/net/dev")?;
        let mut stats_map: HashMap<String, (u64, u64, u64, u64, u64, u64, u64, u64)> = HashMap::new();

        for line in net_dev.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 17 {
                let name = parts[0].trim_end_matches(':');
                stats_map.insert(
                    name.to_string(),
                    (
                        parts[1].parse().unwrap_or(0),  // rx_bytes
                        parts[2].parse().unwrap_or(0),  // rx_packets
                        parts[3].parse().unwrap_or(0),  // rx_errors
                        parts[4].parse().unwrap_or(0),  // rx_dropped
                        parts[9].parse().unwrap_or(0),  // tx_bytes
                        parts[10].parse().unwrap_or(0), // tx_packets
                        parts[11].parse().unwrap_or(0), // tx_errors
                        parts[12].parse().unwrap_or(0), // tx_dropped
                    ),
                );
            }
        }

        if let Ok(entries) = fs::read_dir(net_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let iface_path = entry.path();

                let iface_type = self.classify_interface(&name, &iface_path);
                let is_vpn = matches!(
                    iface_type,
                    InterfaceType::WireGuard | InterfaceType::OpenVPN | InterfaceType::PPP
                );

                let is_up = fs::read_to_string(iface_path.join("operstate"))
                    .map(|s| s.trim() == "up" || s.trim() == "unknown")
                    .unwrap_or(false);

                let mtu = fs::read_to_string(iface_path.join("mtu"))
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);

                let (ipv4_addrs, ipv6_addrs) = Self::get_interface_addresses(&name);

                let stats = stats_map.get(&name).copied().unwrap_or_default();

                interfaces.push(NetworkInterface {
                    name,
                    iface_type,
                    is_vpn,
                    is_up,
                    mtu,
                    ipv4_addrs,
                    ipv6_addrs,
                    rx_bytes: stats.0,
                    rx_packets: stats.1,
                    rx_errors: stats.2,
                    rx_dropped: stats.3,
                    tx_bytes: stats.4,
                    tx_packets: stats.5,
                    tx_errors: stats.6,
                    tx_dropped: stats.7,
                });
            }
        }

        interfaces.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(interfaces)
    }

    fn classify_interface(&self, name: &str, path: &Path) -> InterfaceType {
        if name == "lo" {
            return InterfaceType::Loopback;
        }

        // WireGuard detection
        if name.starts_with("wg") || path.join("device/type").exists() {
            if let Ok(uevent) = fs::read_to_string(path.join("uevent")) {
                if uevent.contains("wireguard") {
                    return InterfaceType::WireGuard;
                }
            }
            // Also check if wg tool recognizes it
            if std::process::Command::new("wg")
                .args(["show", name])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return InterfaceType::WireGuard;
            }
        }

        // TUN/TAP (OpenVPN, etc)
        if name.starts_with("tun") || name.starts_with("tap") {
            return InterfaceType::OpenVPN;
        }

        // PPP interfaces
        if name.starts_with("ppp") {
            return InterfaceType::PPP;
        }

        // Bridge interfaces
        if path.join("bridge").exists() || name.starts_with("br") {
            return InterfaceType::Bridge;
        }

        // Virtual interfaces
        if name.starts_with("veth")
            || name.starts_with("docker")
            || name.starts_with("virbr")
            || name.starts_with("vnet")
        {
            return InterfaceType::Virtual;
        }

        // Physical: has a device symlink pointing to PCI/USB
        if let Ok(device_link) = fs::read_link(path.join("device")) {
            let device_path = device_link.to_string_lossy();
            if device_path.contains("/pci") || device_path.contains("/usb") {
                return InterfaceType::Physical;
            }
        }

        // Check driver for physical hint
        if let Ok(driver_link) = fs::read_link(path.join("device/driver")) {
            let driver = driver_link.file_name().map(|s| s.to_string_lossy().to_string());
            if let Some(drv) = driver {
                if ["e1000", "igb", "ixgbe", "mlx", "r8169", "iwlwifi", "ath", "rtl", "mt76"]
                    .iter()
                    .any(|d| drv.contains(d))
                {
                    return InterfaceType::Physical;
                }
            }
        }

        InterfaceType::Unknown
    }

    fn get_interface_addresses(iface: &str) -> (Vec<String>, Vec<String>) {
        let mut ipv4 = Vec::new();
        let mut ipv6 = Vec::new();

        if let Ok(output) = std::process::Command::new("ip")
            .args(["-o", "addr", "show", iface])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "inet" {
                        if let Some(addr) = parts.get(i + 1) {
                            ipv4.push(addr.to_string());
                        }
                    } else if *part == "inet6" {
                        if let Some(addr) = parts.get(i + 1) {
                            if !addr.starts_with("fe80:") {
                                ipv6.push(addr.to_string());
                            }
                        }
                    }
                }
            }
        }

        (ipv4, ipv6)
    }

    // Routing Table

    fn get_routing_table(&self, vpn_interfaces: &[String]) -> Result<RoutingTable> {
        let mut routes = Vec::new();
        let mut default_interface = None;
        let mut default_via_vpn = false;

        // Parse IPv4 routes
        if let Ok(output) = std::process::Command::new("ip")
            .args(["-4", "route", "show", "table", "all"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(route) = self.parse_route_line(line, vpn_interfaces) {
                    if route.is_default && default_interface.is_none() {
                        default_interface = Some(route.interface.clone());
                        default_via_vpn = route.is_vpn_route;
                    }
                    routes.push(route);
                }
            }
        }

        // Parse IPv6 routes
        if let Ok(output) = std::process::Command::new("ip")
            .args(["-6", "route", "show", "table", "all"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(route) = self.parse_route_line(line, vpn_interfaces) {
                    routes.push(route);
                }
            }
        }

        let vpn_routes: Vec<Route> = routes.iter().filter(|r| r.is_vpn_route).cloned().collect();

        // Detect split tunneling
        let (split_tunnel_detected, split_tunnel_subnets) =
            self.detect_split_tunnel(&routes, vpn_interfaces);

        Ok(RoutingTable {
            default_interface,
            default_via_vpn,
            routes,
            vpn_routes,
            split_tunnel_detected,
            split_tunnel_subnets,
        })
    }

    fn parse_route_line(&self, line: &str, vpn_interfaces: &[String]) -> Option<Route> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let destination = parts[0].to_string();
        let is_default = destination == "default" || destination == "::/0";

        let mut gateway = None;
        let mut interface = String::new();
        let mut metric = 0u32;
        let mut scope = RouteScope::Universe;
        let mut protocol = RouteProtocol::Unknown;

        let mut i = 1;
        while i < parts.len() {
            match parts[i] {
                "via" => {
                    gateway = parts.get(i + 1).map(|s| s.to_string());
                    i += 2;
                }
                "dev" => {
                    interface = parts.get(i + 1).map(|s| s.to_string()).unwrap_or_default();
                    i += 2;
                }
                "metric" => {
                    metric = parts.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    i += 2;
                }
                "scope" => {
                    scope = match parts.get(i + 1) {
                        Some(&"link") => RouteScope::Link,
                        Some(&"host") => RouteScope::Host,
                        Some(&"nowhere") => RouteScope::Nowhere,
                        _ => RouteScope::Universe,
                    };
                    i += 2;
                }
                "proto" => {
                    protocol = match parts.get(i + 1) {
                        Some(&"kernel") => RouteProtocol::Kernel,
                        Some(&"boot") => RouteProtocol::Boot,
                        Some(&"static") => RouteProtocol::Static,
                        Some(&"dhcp") => RouteProtocol::Dhcp,
                        _ => RouteProtocol::Unknown,
                    };
                    i += 2;
                }
                _ => i += 1,
            }
        }

        if interface.is_empty() {
            return None;
        }

        let is_vpn_route = vpn_interfaces.contains(&interface);

        Some(Route {
            destination,
            gateway,
            interface,
            metric,
            scope,
            protocol,
            is_default,
            is_vpn_route,
        })
    }

    fn detect_split_tunnel(
        &self,
        routes: &[Route],
        vpn_interfaces: &[String],
    ) -> (bool, Vec<String>) {
        if vpn_interfaces.is_empty() {
            return (false, Vec::new());
        }

        let has_vpn_default = routes
            .iter()
            .any(|r| r.is_default && r.is_vpn_route);

        let has_non_vpn_default = routes
            .iter()
            .any(|r| r.is_default && !r.is_vpn_route);

        // Split tunnel: VPN has specific routes but not default
        let vpn_specific_routes: Vec<String> = routes
            .iter()
            .filter(|r| r.is_vpn_route && !r.is_default && r.scope == RouteScope::Universe)
            .map(|r| r.destination.clone())
            .collect();

        let split_detected = !has_vpn_default && !vpn_specific_routes.is_empty();

        // Or: both VPN and non-VPN have defaults (unusual but possible)
        let split_detected = split_detected || (has_vpn_default && has_non_vpn_default);

        (split_detected, vpn_specific_routes)
    }

    // WireGuard

    fn get_wireguard_interfaces(&self) -> Vec<WireGuardInterface> {
        let mut interfaces = Vec::new();

        let output = match std::process::Command::new("wg").args(["show", "all", "dump"]).output() {
            Ok(o) if o.status.success() => o,
            _ => return interfaces,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_iface: Option<WireGuardInterface> = None;

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();

            if parts.len() >= 4 && parts.len() <= 5 {
                // Interface line: interface, private_key, public_key, listen_port, fwmark
                if let Some(iface) = current_iface.take() {
                    interfaces.push(iface);
                }

                current_iface = Some(WireGuardInterface {
                    name: parts[0].to_string(),
                    public_key: parts[2].to_string(),
                    listen_port: parts.get(3).and_then(|s| s.parse().ok()),
                    fwmark: parts.get(4).and_then(|s| {
                        if *s == "off" {
                            None
                        } else {
                            s.parse().ok()
                        }
                    }),
                    peers: Vec::new(),
                });
            } else if parts.len() >= 8 {
                // Peer line
                if let Some(ref mut iface) = current_iface {
                    let peer = WireGuardPeer {
                        public_key: parts[1].to_string(),
                        endpoint: if parts[3] == "(none)" {
                            None
                        } else {
                            Some(parts[3].to_string())
                        },
                        allowed_ips: parts[4].split(',').map(String::from).collect(),
                        latest_handshake: parts.get(5).and_then(|s| s.parse().ok()).filter(|&t| t > 0),
                        rx_bytes: parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0),
                        tx_bytes: parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0),
                        persistent_keepalive: parts.get(8).and_then(|s| {
                            if *s == "off" {
                                None
                            } else {
                                s.parse().ok()
                            }
                        }),
                    };
                    iface.peers.push(peer);
                }
            }
        }

        if let Some(iface) = current_iface {
            interfaces.push(iface);
        }

        interfaces
    }

    // VPN Connections (NetworkManager)

    fn get_vpn_connections(&self, interfaces: &[NetworkInterface]) -> Vec<VpnConnection> {
        let mut connections = Vec::new();

        // Try NetworkManager
        if let Ok(output) = std::process::Command::new("nmcli")
            .args(["-t", "-f", "NAME,TYPE,DEVICE,STATE", "connection", "show", "--active"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 4 {
                    let conn_type = parts[1];
                    if conn_type.contains("vpn") || conn_type == "wireguard" {
                        let name = parts[0].to_string();
                        let device = if parts[2].is_empty() {
                            None
                        } else {
                            Some(parts[2].to_string())
                        };

                        let vpn_type = if conn_type == "wireguard" {
                            VpnType::WireGuard
                        } else if conn_type.contains("openvpn") {
                            VpnType::OpenVPN
                        } else if conn_type.contains("ipsec") || conn_type.contains("libreswan") {
                            VpnType::IPSec
                        } else if conn_type.contains("pptp") {
                            VpnType::PPTP
                        } else if conn_type.contains("l2tp") {
                            VpnType::L2TP
                        } else {
                            VpnType::Unknown
                        };

                        let state = match parts[3] {
                            "activated" => VpnState::Connected,
                            "activating" => VpnState::Connecting,
                            "deactivated" | "deactivating" => VpnState::Disconnected,
                            _ => VpnState::Unknown,
                        };

                        let (rx_bytes, tx_bytes, local_ip) = device
                            .as_ref()
                            .and_then(|d| interfaces.iter().find(|i| &i.name == d))
                            .map(|i| {
                                (
                                    i.rx_bytes,
                                    i.tx_bytes,
                                    i.ipv4_addrs.first().cloned(),
                                )
                            })
                            .unwrap_or((0, 0, None));

                        connections.push(VpnConnection {
                            name,
                            vpn_type,
                            state,
                            interface: device,
                            server: None, // Would need additional nmcli query
                            local_ip,
                            connected_since: None,
                            rx_bytes,
                            tx_bytes,
                        });
                    }
                }
            }
        }

        // Add standalone WireGuard interfaces not managed by NM
        for iface in interfaces.iter().filter(|i| i.iface_type == InterfaceType::WireGuard) {
            if !connections.iter().any(|c| c.interface.as_ref() == Some(&iface.name)) {
                connections.push(VpnConnection {
                    name: iface.name.clone(),
                    vpn_type: VpnType::WireGuard,
                    state: if iface.is_up {
                        VpnState::Connected
                    } else {
                        VpnState::Disconnected
                    },
                    interface: Some(iface.name.clone()),
                    server: None,
                    local_ip: iface.ipv4_addrs.first().cloned(),
                    connected_since: None,
                    rx_bytes: iface.rx_bytes,
                    tx_bytes: iface.tx_bytes,
                });
            }
        }

        connections
    }

    // Bandwidth Split

    fn calculate_bandwidth_split(&mut self, interfaces: &[NetworkInterface]) -> BandwidthSplit {
        let now = Instant::now();
        let mut current_stats: HashMap<String, (u64, u64)> = HashMap::new();

        let mut vpn_rx: u64 = 0;
        let mut vpn_tx: u64 = 0;
        let mut direct_rx: u64 = 0;
        let mut direct_tx: u64 = 0;

        for iface in interfaces {
            if iface.iface_type == InterfaceType::Loopback {
                continue;
            }

            current_stats.insert(iface.name.clone(), (iface.rx_bytes, iface.tx_bytes));

            if iface.is_vpn {
                vpn_rx += iface.rx_bytes;
                vpn_tx += iface.tx_bytes;
            } else if iface.iface_type == InterfaceType::Physical {
                direct_rx += iface.rx_bytes;
                direct_tx += iface.tx_bytes;
            }
        }

        let (vpn_rx_rate, vpn_tx_rate, direct_rx_rate, direct_tx_rate) =
            if let Some((prev_time, prev_stats)) = &self.prev_stats {
                let elapsed = now.duration_since(*prev_time).as_secs_f64();
                if elapsed > 0.0 {
                    let mut vpn_rx_delta = 0u64;
                    let mut vpn_tx_delta = 0u64;
                    let mut direct_rx_delta = 0u64;
                    let mut direct_tx_delta = 0u64;

                    for iface in interfaces {
                        if let Some((prev_rx, prev_tx)) = prev_stats.get(&iface.name) {
                            let rx_delta = iface.rx_bytes.saturating_sub(*prev_rx);
                            let tx_delta = iface.tx_bytes.saturating_sub(*prev_tx);

                            if iface.is_vpn {
                                vpn_rx_delta += rx_delta;
                                vpn_tx_delta += tx_delta;
                            } else if iface.iface_type == InterfaceType::Physical {
                                direct_rx_delta += rx_delta;
                                direct_tx_delta += tx_delta;
                            }
                        }
                    }

                    (
                        vpn_rx_delta as f64 / elapsed,
                        vpn_tx_delta as f64 / elapsed,
                        direct_rx_delta as f64 / elapsed,
                        direct_tx_delta as f64 / elapsed,
                    )
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                }
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

        self.prev_stats = Some((now, current_stats));

        let total = vpn_rx + vpn_tx + direct_rx + direct_tx;
        let vpn_total = vpn_rx + vpn_tx;
        let vpn_percentage = if total > 0 {
            (vpn_total as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        BandwidthSplit {
            vpn_rx_bytes: vpn_rx,
            vpn_tx_bytes: vpn_tx,
            direct_rx_bytes: direct_rx,
            direct_tx_bytes: direct_tx,
            vpn_rx_rate,
            vpn_tx_rate,
            direct_rx_rate,
            direct_tx_rate,
            vpn_percentage,
        }
    }

    // Active Connections

    fn refresh_inode_map(&mut self) {
        self.inode_to_pid.clear();

        let proc_path = Path::new("/proc");
        if let Ok(entries) = fs::read_dir(proc_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if let Ok(pid) = name.to_string_lossy().parse::<u32>() {
                    let fd_path = proc_path.join(name.to_string_lossy().as_ref()).join("fd");
                    if let Ok(fds) = fs::read_dir(&fd_path) {
                        let proc_name = fs::read_to_string(
                            proc_path.join(pid.to_string()).join("comm"),
                        )
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                        for fd in fds.flatten() {
                            if let Ok(link) = fs::read_link(fd.path()) {
                                let link_str = link.to_string_lossy();
                                if link_str.starts_with("socket:[") {
                                    if let Some(inode_str) = link_str
                                        .strip_prefix("socket:[")
                                        .and_then(|s| s.strip_suffix(']'))
                                    {
                                        if let Ok(inode) = inode_str.parse::<u64>() {
                                            self.inode_to_pid
                                                .insert(inode, (pid, proc_name.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_active_connections(&self, routing: &RoutingTable) -> Result<Vec<ActiveConnection>> {
        let mut connections = Vec::new();

        // TCP connections
        if let Ok(tcp) = fs::read_to_string("/proc/net/tcp") {
            for line in tcp.lines().skip(1) {
                if let Some(conn) = self.parse_proc_net_line(line, ConnectionProtocol::Tcp, routing)
                {
                    connections.push(conn);
                }
            }
        }

        // TCP6 connections
        if let Ok(tcp6) = fs::read_to_string("/proc/net/tcp6") {
            for line in tcp6.lines().skip(1) {
                if let Some(conn) =
                    self.parse_proc_net_line(line, ConnectionProtocol::Tcp6, routing)
                {
                    connections.push(conn);
                }
            }
        }

        // UDP connections
        if let Ok(udp) = fs::read_to_string("/proc/net/udp") {
            for line in udp.lines().skip(1) {
                if let Some(conn) = self.parse_proc_net_line(line, ConnectionProtocol::Udp, routing)
                {
                    connections.push(conn);
                }
            }
        }

        // UDP6 connections
        if let Ok(udp6) = fs::read_to_string("/proc/net/udp6") {
            for line in udp6.lines().skip(1) {
                if let Some(conn) =
                    self.parse_proc_net_line(line, ConnectionProtocol::Udp6, routing)
                {
                    connections.push(conn);
                }
            }
        }

        // Filter out listening sockets and sort by state
        connections.retain(|c| {
            c.state != ConnectionState::Listen
                && c.state != ConnectionState::Close
                && c.remote_addr != "0.0.0.0"
                && c.remote_addr != "::"
        });

        connections.sort_by(|a, b| {
            a.process_name
                .as_ref()
                .unwrap_or(&String::new())
                .cmp(b.process_name.as_ref().unwrap_or(&String::new()))
        });

        Ok(connections)
    }

    fn parse_proc_net_line(
        &self,
        line: &str,
        protocol: ConnectionProtocol,
        routing: &RoutingTable,
    ) -> Option<ActiveConnection> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            return None;
        }

        let local = parts[1];
        let remote = parts[2];
        let state_hex = parts[3];
        let inode: u64 = parts[9].parse().ok()?;

        let (local_addr, local_port) = Self::parse_hex_addr(local, protocol)?;
        let (remote_addr, remote_port) = Self::parse_hex_addr(remote, protocol)?;

        let state = match u8::from_str_radix(state_hex, 16).unwrap_or(0) {
            0x01 => ConnectionState::Established,
            0x02 => ConnectionState::SynSent,
            0x03 => ConnectionState::SynRecv,
            0x04 => ConnectionState::FinWait1,
            0x05 => ConnectionState::FinWait2,
            0x06 => ConnectionState::TimeWait,
            0x07 => ConnectionState::Close,
            0x08 => ConnectionState::CloseWait,
            0x09 => ConnectionState::LastAck,
            0x0A => ConnectionState::Listen,
            0x0B => ConnectionState::Closing,
            _ => ConnectionState::Unknown,
        };

        let (pid, process_name) = self
            .inode_to_pid
            .get(&inode)
            .map(|(p, n)| (Some(*p), Some(n.clone())))
            .unwrap_or((None, None));

        // Determine interface hint based on routing
        let (interface_hint, via_vpn) = self.route_lookup(&remote_addr, routing);

        Some(ActiveConnection {
            protocol,
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state,
            inode,
            pid,
            process_name,
            interface_hint,
            via_vpn,
        })
    }

    fn parse_hex_addr(hex: &str, protocol: ConnectionProtocol) -> Option<(String, u16)> {
        let parts: Vec<&str> = hex.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let port = u16::from_str_radix(parts[1], 16).ok()?;

        let addr = match protocol {
            ConnectionProtocol::Tcp | ConnectionProtocol::Udp => {
                let bytes = u32::from_str_radix(parts[0], 16).ok()?;
                Ipv4Addr::from(bytes.swap_bytes()).to_string()
            }
            ConnectionProtocol::Tcp6 | ConnectionProtocol::Udp6 => {
                if parts[0].len() != 32 {
                    return None;
                }
                let mut bytes = [0u8; 16];
                for i in 0..4 {
                    let chunk = &parts[0][i * 8..(i + 1) * 8];
                    let val = u32::from_str_radix(chunk, 16).ok()?;
                    bytes[i * 4..(i + 1) * 4].copy_from_slice(&val.to_le_bytes());
                }
                Ipv6Addr::from(bytes).to_string()
            }
        };

        Some((addr, port))
    }

    fn route_lookup(&self, dest: &str, routing: &RoutingTable) -> (Option<String>, Option<bool>) {
        // Parse destination IP
        let dest_ip: IpAddr = match dest.parse() {
            Ok(ip) => ip,
            Err(_) => return (None, None),
        };

        // Find most specific matching route
        let mut best_match: Option<&Route> = None;
        let mut best_prefix_len = 0u8;

        for route in &routing.routes {
            if route.scope == RouteScope::Link || route.scope == RouteScope::Host {
                continue;
            }

            let (route_net, prefix_len) = if route.destination == "default" {
                match dest_ip {
                    IpAddr::V4(_) => (IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
                    IpAddr::V6(_) => (IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), 0),
                }
            } else {
                match Self::parse_cidr(&route.destination) {
                    Some(r) => r,
                    None => continue,
                }
            };

            // Check IP version match
            match (&dest_ip, &route_net) {
                (IpAddr::V4(_), IpAddr::V6(_)) | (IpAddr::V6(_), IpAddr::V4(_)) => continue,
                _ => {}
            }

            if Self::ip_in_network(&dest_ip, &route_net, prefix_len) {
                if best_match.is_none() || prefix_len > best_prefix_len {
                    best_match = Some(route);
                    best_prefix_len = prefix_len;
                }
            }
        }

        match best_match {
            Some(route) => (Some(route.interface.clone()), Some(route.is_vpn_route)),
            None => (routing.default_interface.clone(), Some(routing.default_via_vpn)),
        }
    }

    fn parse_cidr(cidr: &str) -> Option<(IpAddr, u8)> {
        let parts: Vec<&str> = cidr.split('/').collect();
        let addr: IpAddr = parts[0].parse().ok()?;
        let prefix: u8 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(match addr {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        });
        Some((addr, prefix))
    }

    fn ip_in_network(ip: &IpAddr, network: &IpAddr, prefix_len: u8) -> bool {
        match (ip, network) {
            (IpAddr::V4(ip), IpAddr::V4(net)) => {
                if prefix_len == 0 {
                    return true;
                }
                let mask = !0u32 << (32 - prefix_len);
                (u32::from(*ip) & mask) == (u32::from(*net) & mask)
            }
            (IpAddr::V6(ip), IpAddr::V6(net)) => {
                if prefix_len == 0 {
                    return true;
                }
                let ip_bytes = ip.octets();
                let net_bytes = net.octets();
                let full_bytes = (prefix_len / 8) as usize;
                let remaining_bits = prefix_len % 8;

                if ip_bytes[..full_bytes] != net_bytes[..full_bytes] {
                    return false;
                }

                if remaining_bits > 0 && full_bytes < 16 {
                    let mask = !0u8 << (8 - remaining_bits);
                    (ip_bytes[full_bytes] & mask) == (net_bytes[full_bytes] & mask)
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    // DNS State

    fn get_dns_state(&self, vpn_interfaces: &[String]) -> (Vec<String>, bool) {
        let mut dns_servers = Vec::new();
        let mut dns_via_vpn = false;

        // Check systemd-resolved first
        if let Ok(output) = std::process::Command::new("resolvectl")
            .args(["status", "--no-pager"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_interface = String::new();

            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("Link ") || line.contains("(") && line.contains(")") {
                    // Extract interface name
                    if let Some(start) = line.find('(') {
                        if let Some(end) = line.find(')') {
                            current_interface = line[start + 1..end].to_string();
                        }
                    }
                } else if line.starts_with("DNS Servers:") || line.starts_with("Current DNS Server:") {
                    let server_part = line.split(':').nth(1).unwrap_or("").trim();
                    if !server_part.is_empty() {
                        dns_servers.push(server_part.to_string());
                        if vpn_interfaces.contains(&current_interface) {
                            dns_via_vpn = true;
                        }
                    }
                }
            }
        }

        // Fallback to /etc/resolv.conf
        if dns_servers.is_empty() {
            if let Ok(resolv) = fs::read_to_string("/etc/resolv.conf") {
                for line in resolv.lines() {
                    if line.starts_with("nameserver") {
                        if let Some(server) = line.split_whitespace().nth(1) {
                            dns_servers.push(server.to_string());
                        }
                    }
                }
            }
        }

        (dns_servers, dns_via_vpn)
    }
}
