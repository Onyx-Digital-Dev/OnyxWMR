//! System telemetry backend
//!
//! Comprehensive system monitoring: CPU, memory, disk, network, GPU, processes, sensors.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

// CPU Telemetry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores_physical: u32,
    pub cores_logical: u32,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUsage {
    pub total_percent: f32,
    pub per_core: Vec<f32>,
    pub user_percent: f32,
    pub system_percent: f32,
    pub idle_percent: f32,
    pub iowait_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuFrequency {
    pub current_mhz: u32,
    pub min_mhz: u32,
    pub max_mhz: u32,
    pub per_core_mhz: Vec<u32>,
}

// Memory Telemetry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub used_percent: f32,
    pub buffers_bytes: u64,
    pub cached_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_percent: f32,
}

// Disk Telemetry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub device: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub used_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIo {
    pub device: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
}

// Network Telemetry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: Option<String>,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
    pub is_up: bool,
    pub speed_mbps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIo {
    pub interface: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

// GPU Telemetry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub driver: String,
    pub vram_total_mb: Option<u32>,
    pub vram_used_mb: Option<u32>,
    pub utilization_percent: Option<u8>,
    pub temperature_celsius: Option<u8>,
    pub power_watts: Option<f32>,
}

// Process Telemetry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub user: String,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_bytes: u64,
    pub state: ProcessState,
    pub threads: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Unknown,
}

// Sensor Telemetry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Temperature {
    pub label: String,
    pub current_celsius: f32,
    pub high_celsius: Option<f32>,
    pub critical_celsius: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanSpeed {
    pub label: String,
    pub rpm: u32,
}

// System Overview

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemOverview {
    pub hostname: String,
    pub kernel: String,
    pub uptime_seconds: u64,
    pub load_average: (f32, f32, f32),
    pub boot_time: u64,
}

// Full Telemetry Snapshot

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub timestamp: u64,
    pub system: SystemOverview,
    pub cpu_info: CpuInfo,
    pub cpu_usage: CpuUsage,
    pub cpu_frequency: CpuFrequency,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub disk_io: Vec<DiskIo>,
    pub network_interfaces: Vec<NetworkInterface>,
    pub network_io: Vec<NetworkIo>,
    pub gpus: Vec<GpuInfo>,
    pub temperatures: Vec<Temperature>,
    pub fans: Vec<FanSpeed>,
    pub top_processes: Vec<ProcessInfo>,
}

// Telemetry Backend

pub struct TelemetryBackend {
    prev_cpu_stats: Option<(Instant, Vec<CpuStat>)>,
    prev_disk_io: Option<(Instant, HashMap<String, DiskIo>)>,
    prev_net_io: Option<(Instant, HashMap<String, NetworkIo>)>,
    prev_proc_times: HashMap<u32, (u64, u64)>,
}

#[derive(Clone)]
struct CpuStat {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl Default for TelemetryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryBackend {
    pub fn new() -> Self {
        Self {
            prev_cpu_stats: None,
            prev_disk_io: None,
            prev_net_io: None,
            prev_proc_times: HashMap::new(),
        }
    }

    pub fn snapshot(&mut self) -> Result<TelemetrySnapshot> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(TelemetrySnapshot {
            timestamp,
            system: self.system_overview()?,
            cpu_info: Self::cpu_info()?,
            cpu_usage: self.cpu_usage()?,
            cpu_frequency: Self::cpu_frequency()?,
            memory: Self::memory_info()?,
            disks: Self::disk_info()?,
            disk_io: self.disk_io()?,
            network_interfaces: Self::network_interfaces()?,
            network_io: self.network_io()?,
            gpus: Self::gpu_info(),
            temperatures: Self::temperatures(),
            fans: Self::fan_speeds(),
            top_processes: self.top_processes(10)?,
        })
    }

    // System Overview

    fn system_overview(&self) -> Result<SystemOverview> {
        let hostname = fs::read_to_string("/etc/hostname")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let kernel = fs::read_to_string("/proc/version")
            .map(|s| {
                s.split_whitespace()
                    .nth(2)
                    .unwrap_or("unknown")
                    .to_string()
            })
            .unwrap_or_else(|_| "unknown".to_string());

        let uptime = fs::read_to_string("/proc/uptime")
            .map(|s| {
                s.split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0) as u64
            })
            .unwrap_or(0);

        let load_average = fs::read_to_string("/proc/loadavg")
            .map(|s| {
                let parts: Vec<f32> = s
                    .split_whitespace()
                    .take(3)
                    .filter_map(|v| v.parse().ok())
                    .collect();
                (
                    *parts.first().unwrap_or(&0.0),
                    *parts.get(1).unwrap_or(&0.0),
                    *parts.get(2).unwrap_or(&0.0),
                )
            })
            .unwrap_or((0.0, 0.0, 0.0));

        let boot_time = fs::read_to_string("/proc/stat")
            .map(|s| {
                s.lines()
                    .find(|l| l.starts_with("btime"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        Ok(SystemOverview {
            hostname,
            kernel,
            uptime_seconds: uptime,
            load_average,
            boot_time,
        })
    }

    // CPU Information

    fn cpu_info() -> Result<CpuInfo> {
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;

        let model = cpuinfo
            .lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let cores_logical = cpuinfo
            .lines()
            .filter(|l| l.starts_with("processor"))
            .count() as u32;

        let cores_physical = cpuinfo
            .lines()
            .find(|l| l.starts_with("cpu cores"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(cores_logical);

        Ok(CpuInfo {
            model,
            cores_physical,
            cores_logical,
            architecture: std::env::consts::ARCH.to_string(),
        })
    }

    fn cpu_usage(&mut self) -> Result<CpuUsage> {
        let stats = Self::read_cpu_stats()?;
        let now = Instant::now();

        let usage = if let Some((prev_time, prev_stats)) = &self.prev_cpu_stats {
            let elapsed = now.duration_since(*prev_time);
            if elapsed.as_millis() > 0 {
                Self::calculate_cpu_usage(prev_stats, &stats)
            } else {
                Self::zero_cpu_usage(stats.len().saturating_sub(1))
            }
        } else {
            Self::zero_cpu_usage(stats.len().saturating_sub(1))
        };

        self.prev_cpu_stats = Some((now, stats));
        Ok(usage)
    }

    fn read_cpu_stats() -> Result<Vec<CpuStat>> {
        let stat = fs::read_to_string("/proc/stat")?;
        let mut stats = Vec::new();

        for line in stat.lines() {
            if line.starts_with("cpu") {
                let parts: Vec<u64> = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|v| v.parse().ok())
                    .collect();

                if parts.len() >= 8 {
                    stats.push(CpuStat {
                        user: parts[0],
                        nice: parts[1],
                        system: parts[2],
                        idle: parts[3],
                        iowait: parts[4],
                        irq: parts[5],
                        softirq: parts[6],
                        steal: parts[7],
                    });
                }
            }
        }

        Ok(stats)
    }

    fn calculate_cpu_usage(prev: &[CpuStat], curr: &[CpuStat]) -> CpuUsage {
        let calc_single = |p: &CpuStat, c: &CpuStat| -> (f32, f32, f32, f32, f32) {
            let prev_idle = p.idle + p.iowait;
            let curr_idle = c.idle + c.iowait;

            let prev_total = p.user + p.nice + p.system + p.idle + p.iowait + p.irq + p.softirq + p.steal;
            let curr_total = c.user + c.nice + c.system + c.idle + c.iowait + c.irq + c.softirq + c.steal;

            let total_delta = curr_total.saturating_sub(prev_total) as f32;
            let idle_delta = curr_idle.saturating_sub(prev_idle) as f32;

            if total_delta > 0.0 {
                let usage = 100.0 * (1.0 - idle_delta / total_delta);
                let user = 100.0 * (c.user.saturating_sub(p.user)) as f32 / total_delta;
                let system = 100.0 * (c.system.saturating_sub(p.system)) as f32 / total_delta;
                let idle = 100.0 * idle_delta / total_delta;
                let iowait = 100.0 * (c.iowait.saturating_sub(p.iowait)) as f32 / total_delta;
                (usage, user, system, idle, iowait)
            } else {
                (0.0, 0.0, 0.0, 100.0, 0.0)
            }
        };

        let (total, user, system, idle, iowait) = if !prev.is_empty() && !curr.is_empty() {
            calc_single(&prev[0], &curr[0])
        } else {
            (0.0, 0.0, 0.0, 100.0, 0.0)
        };

        let per_core: Vec<f32> = prev
            .iter()
            .skip(1)
            .zip(curr.iter().skip(1))
            .map(|(p, c)| calc_single(p, c).0)
            .collect();

        CpuUsage {
            total_percent: total,
            per_core,
            user_percent: user,
            system_percent: system,
            idle_percent: idle,
            iowait_percent: iowait,
        }
    }

    fn zero_cpu_usage(cores: usize) -> CpuUsage {
        CpuUsage {
            total_percent: 0.0,
            per_core: vec![0.0; cores],
            user_percent: 0.0,
            system_percent: 0.0,
            idle_percent: 100.0,
            iowait_percent: 0.0,
        }
    }

    fn cpu_frequency() -> Result<CpuFrequency> {
        let base_path = Path::new("/sys/devices/system/cpu");
        let mut per_core: Vec<u32> = Vec::new();
        let mut min_mhz = u32::MAX;
        let mut max_mhz = 0u32;

        for i in 0..128 {
            let freq_path = base_path.join(format!("cpu{}/cpufreq/scaling_cur_freq", i));
            if let Ok(freq) = fs::read_to_string(&freq_path) {
                if let Ok(khz) = freq.trim().parse::<u32>() {
                    per_core.push(khz / 1000);
                }
            } else {
                break;
            }

            if let Ok(min) = fs::read_to_string(base_path.join(format!("cpu{}/cpufreq/scaling_min_freq", i))) {
                if let Ok(khz) = min.trim().parse::<u32>() {
                    min_mhz = min_mhz.min(khz / 1000);
                }
            }

            if let Ok(max) = fs::read_to_string(base_path.join(format!("cpu{}/cpufreq/scaling_max_freq", i))) {
                if let Ok(khz) = max.trim().parse::<u32>() {
                    max_mhz = max_mhz.max(khz / 1000);
                }
            }
        }

        let current_mhz = per_core.iter().sum::<u32>() / per_core.len().max(1) as u32;

        Ok(CpuFrequency {
            current_mhz,
            min_mhz: if min_mhz == u32::MAX { 0 } else { min_mhz },
            max_mhz,
            per_core_mhz: per_core,
        })
    }

    // Memory Information

    fn memory_info() -> Result<MemoryInfo> {
        let meminfo = fs::read_to_string("/proc/meminfo")?;
        let mut values: HashMap<&str, u64> = HashMap::new();

        for line in meminfo.lines() {
            if let Some((key, rest)) = line.split_once(':') {
                if let Some(val) = rest.split_whitespace().next() {
                    if let Ok(kb) = val.parse::<u64>() {
                        values.insert(key, kb * 1024);
                    }
                }
            }
        }

        let total = *values.get("MemTotal").unwrap_or(&0);
        let available = *values.get("MemAvailable").unwrap_or(&0);
        let buffers = *values.get("Buffers").unwrap_or(&0);
        let cached = *values.get("Cached").unwrap_or(&0);
        let swap_total = *values.get("SwapTotal").unwrap_or(&0);
        let swap_free = *values.get("SwapFree").unwrap_or(&0);

        let used = total.saturating_sub(available);
        let swap_used = swap_total.saturating_sub(swap_free);

        Ok(MemoryInfo {
            total_bytes: total,
            available_bytes: available,
            used_bytes: used,
            used_percent: if total > 0 { (used as f32 / total as f32) * 100.0 } else { 0.0 },
            buffers_bytes: buffers,
            cached_bytes: cached,
            swap_total_bytes: swap_total,
            swap_used_bytes: swap_used,
            swap_percent: if swap_total > 0 { (swap_used as f32 / swap_total as f32) * 100.0 } else { 0.0 },
        })
    }

    // Disk Information

    fn disk_info() -> Result<Vec<DiskInfo>> {
        let mounts = fs::read_to_string("/proc/mounts")?;
        let mut disks = Vec::new();

        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            let device = parts[0];
            let mount_point = parts[1];
            let filesystem = parts[2];

            // Skip virtual filesystems
            if !device.starts_with("/dev/") || filesystem == "squashfs" {
                continue;
            }

            if let Ok(stat) = nix::sys::statvfs::statvfs(mount_point) {
                let block_size = stat.block_size() as u64;
                let total = stat.blocks() * block_size;
                let available = stat.blocks_available() * block_size;
                let used = total.saturating_sub(stat.blocks_free() * block_size);

                disks.push(DiskInfo {
                    device: device.to_string(),
                    mount_point: mount_point.to_string(),
                    filesystem: filesystem.to_string(),
                    total_bytes: total,
                    used_bytes: used,
                    available_bytes: available,
                    used_percent: if total > 0 { (used as f32 / total as f32) * 100.0 } else { 0.0 },
                });
            }
        }

        Ok(disks)
    }

    fn disk_io(&mut self) -> Result<Vec<DiskIo>> {
        let diskstats = fs::read_to_string("/proc/diskstats")?;
        let mut current: HashMap<String, DiskIo> = HashMap::new();

        for line in diskstats.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 14 {
                continue;
            }

            let name = parts[2];
            // Skip partitions, only show whole disks
            if name.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                if !name.starts_with("nvme") || !name.contains("n1") || name.contains("p") {
                    continue;
                }
            }

            let read_ops: u64 = parts[3].parse().unwrap_or(0);
            let read_sectors: u64 = parts[5].parse().unwrap_or(0);
            let write_ops: u64 = parts[7].parse().unwrap_or(0);
            let write_sectors: u64 = parts[9].parse().unwrap_or(0);

            current.insert(
                name.to_string(),
                DiskIo {
                    device: name.to_string(),
                    read_bytes: read_sectors * 512,
                    write_bytes: write_sectors * 512,
                    read_ops,
                    write_ops,
                },
            );
        }

        let result: Vec<DiskIo> = current.values().cloned().collect();
        self.prev_disk_io = Some((Instant::now(), current));
        Ok(result)
    }

    // Network Information

    fn network_interfaces() -> Result<Vec<NetworkInterface>> {
        let mut interfaces = Vec::new();
        let net_path = Path::new("/sys/class/net");

        if let Ok(entries) = fs::read_dir(net_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "lo" {
                    continue;
                }

                let iface_path = entry.path();

                let is_up = fs::read_to_string(iface_path.join("operstate"))
                    .map(|s| s.trim() == "up")
                    .unwrap_or(false);

                let mac = fs::read_to_string(iface_path.join("address"))
                    .map(|s| s.trim().to_string())
                    .ok();

                let speed = fs::read_to_string(iface_path.join("speed"))
                    .ok()
                    .and_then(|s| s.trim().parse().ok());

                let (ipv4, ipv6) = Self::get_interface_addresses(&name);

                interfaces.push(NetworkInterface {
                    name,
                    mac_address: mac,
                    ipv4,
                    ipv6,
                    is_up,
                    speed_mbps: speed,
                });
            }
        }

        Ok(interfaces)
    }

    fn get_interface_addresses(iface: &str) -> (Vec<String>, Vec<String>) {
        let mut ipv4 = Vec::new();
        let mut ipv6 = Vec::new();

        if let Ok(output) = std::process::Command::new("ip")
            .args(["addr", "show", iface])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("inet ") {
                    if let Some(addr) = line.split_whitespace().nth(1) {
                        ipv4.push(addr.to_string());
                    }
                } else if line.starts_with("inet6 ") {
                    if let Some(addr) = line.split_whitespace().nth(1) {
                        if !addr.starts_with("fe80:") {
                            ipv6.push(addr.to_string());
                        }
                    }
                }
            }
        }

        (ipv4, ipv6)
    }

    fn network_io(&mut self) -> Result<Vec<NetworkIo>> {
        let netdev = fs::read_to_string("/proc/net/dev")?;
        let mut current: HashMap<String, NetworkIo> = HashMap::new();

        for line in netdev.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 11 {
                continue;
            }

            let name = parts[0].trim_end_matches(':');
            if name == "lo" {
                continue;
            }

            current.insert(
                name.to_string(),
                NetworkIo {
                    interface: name.to_string(),
                    rx_bytes: parts[1].parse().unwrap_or(0),
                    rx_packets: parts[2].parse().unwrap_or(0),
                    rx_errors: parts[3].parse().unwrap_or(0),
                    tx_bytes: parts[9].parse().unwrap_or(0),
                    tx_packets: parts[10].parse().unwrap_or(0),
                    tx_errors: parts[11].parse().unwrap_or(0),
                },
            );
        }

        let result: Vec<NetworkIo> = current.values().cloned().collect();
        self.prev_net_io = Some((Instant::now(), current));
        Ok(result)
    }

    // GPU Information

    fn gpu_info() -> Vec<GpuInfo> {
        let mut gpus = Vec::new();

        // Try NVIDIA first
        if let Some(nvidia) = Self::nvidia_gpu() {
            gpus.push(nvidia);
        }

        // Try AMD
        gpus.extend(Self::amd_gpus());

        // Try Intel
        if let Some(intel) = Self::intel_gpu() {
            gpus.push(intel);
        }

        // Fallback to DRM info
        if gpus.is_empty() {
            gpus.extend(Self::drm_gpus());
        }

        gpus
    }

    fn nvidia_gpu() -> Option<GpuInfo> {
        let output = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name,driver_version,memory.total,memory.used,utilization.gpu,temperature.gpu,power.draw", "--format=csv,noheader,nounits"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split(", ").collect();

        if parts.len() >= 7 {
            Some(GpuInfo {
                name: parts[0].to_string(),
                vendor: "NVIDIA".to_string(),
                driver: parts[1].to_string(),
                vram_total_mb: parts[2].parse().ok(),
                vram_used_mb: parts[3].parse().ok(),
                utilization_percent: parts[4].parse().ok(),
                temperature_celsius: parts[5].parse().ok(),
                power_watts: parts[6].parse().ok(),
            })
        } else {
            None
        }
    }

    fn amd_gpus() -> Vec<GpuInfo> {
        let mut gpus = Vec::new();
        let hwmon_path = Path::new("/sys/class/hwmon");

        if let Ok(entries) = fs::read_dir(hwmon_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = fs::read_to_string(path.join("name"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                if name == "amdgpu" {
                    let temp = fs::read_to_string(path.join("temp1_input"))
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                        .map(|t| (t / 1000) as u8);

                    let power = fs::read_to_string(path.join("power1_average"))
                        .ok()
                        .and_then(|s| s.trim().parse::<u64>().ok())
                        .map(|p| p as f32 / 1_000_000.0);

                    gpus.push(GpuInfo {
                        name: "AMD GPU".to_string(),
                        vendor: "AMD".to_string(),
                        driver: "amdgpu".to_string(),
                        vram_total_mb: None,
                        vram_used_mb: None,
                        utilization_percent: None,
                        temperature_celsius: temp,
                        power_watts: power,
                    });
                }
            }
        }

        gpus
    }

    fn intel_gpu() -> Option<GpuInfo> {
        let drm_path = Path::new("/sys/class/drm");

        if let Ok(entries) = fs::read_dir(drm_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("card") && !name.contains("-") {
                    let device_path = entry.path().join("device");
                    if let Ok(vendor) = fs::read_to_string(device_path.join("vendor")) {
                        if vendor.trim() == "0x8086" {
                            return Some(GpuInfo {
                                name: "Intel Integrated Graphics".to_string(),
                                vendor: "Intel".to_string(),
                                driver: "i915".to_string(),
                                vram_total_mb: None,
                                vram_used_mb: None,
                                utilization_percent: None,
                                temperature_celsius: None,
                                power_watts: None,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    fn drm_gpus() -> Vec<GpuInfo> {
        let mut gpus = Vec::new();
        let drm_path = Path::new("/sys/class/drm");

        if let Ok(entries) = fs::read_dir(drm_path) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("card") && !name.contains("-") {
                    let device_path = entry.path().join("device");

                    let vendor = fs::read_to_string(device_path.join("vendor"))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| "Unknown".to_string());

                    let driver = fs::read_link(device_path.join("driver"))
                        .ok()
                        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "Unknown".to_string());

                    gpus.push(GpuInfo {
                        name: format!("GPU {}", name),
                        vendor,
                        driver,
                        vram_total_mb: None,
                        vram_used_mb: None,
                        utilization_percent: None,
                        temperature_celsius: None,
                        power_watts: None,
                    });
                }
            }
        }

        gpus
    }

    // Temperature Sensors

    fn temperatures() -> Vec<Temperature> {
        let mut temps = Vec::new();
        let hwmon_path = Path::new("/sys/class/hwmon");

        if let Ok(entries) = fs::read_dir(hwmon_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let sensor_name = fs::read_to_string(path.join("name"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());

                for i in 1..20 {
                    let temp_path = path.join(format!("temp{}_input", i));
                    if !temp_path.exists() {
                        break;
                    }

                    let label = fs::read_to_string(path.join(format!("temp{}_label", i)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("{} temp{}", sensor_name, i));

                    let current = fs::read_to_string(&temp_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<i32>().ok())
                        .map(|t| t as f32 / 1000.0)
                        .unwrap_or(0.0);

                    let high = fs::read_to_string(path.join(format!("temp{}_max", i)))
                        .ok()
                        .and_then(|s| s.trim().parse::<i32>().ok())
                        .map(|t| t as f32 / 1000.0);

                    let critical = fs::read_to_string(path.join(format!("temp{}_crit", i)))
                        .ok()
                        .and_then(|s| s.trim().parse::<i32>().ok())
                        .map(|t| t as f32 / 1000.0);

                    temps.push(Temperature {
                        label,
                        current_celsius: current,
                        high_celsius: high,
                        critical_celsius: critical,
                    });
                }
            }
        }

        temps
    }

    fn fan_speeds() -> Vec<FanSpeed> {
        let mut fans = Vec::new();
        let hwmon_path = Path::new("/sys/class/hwmon");

        if let Ok(entries) = fs::read_dir(hwmon_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let sensor_name = fs::read_to_string(path.join("name"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());

                for i in 1..10 {
                    let fan_path = path.join(format!("fan{}_input", i));
                    if !fan_path.exists() {
                        break;
                    }

                    let label = fs::read_to_string(path.join(format!("fan{}_label", i)))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("{} fan{}", sensor_name, i));

                    let rpm = fs::read_to_string(&fan_path)
                        .ok()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);

                    if rpm > 0 {
                        fans.push(FanSpeed { label, rpm });
                    }
                }
            }
        }

        fans
    }

    // Process Information

    fn top_processes(&mut self, limit: usize) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();
        let proc_path = Path::new("/proc");

        let total_mem = Self::memory_info().map(|m| m.total_bytes).unwrap_or(1);
        let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as u64;
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;

        if let Ok(entries) = fs::read_dir(proc_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if let Ok(pid) = name.to_string_lossy().parse::<u32>() {
                    if let Some(proc) = self.read_process(pid, total_mem, ticks_per_sec, page_size) {
                        processes.push(proc);
                    }
                }
            }
        }

        // Sort by CPU usage descending
        processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
        processes.truncate(limit);

        Ok(processes)
    }

    fn read_process(&mut self, pid: u32, total_mem: u64, ticks: u64, page_size: u64) -> Option<ProcessInfo> {
        let proc_path = Path::new("/proc").join(pid.to_string());

        let stat = fs::read_to_string(proc_path.join("stat")).ok()?;
        let status = fs::read_to_string(proc_path.join("status")).ok()?;
        let cmdline = fs::read_to_string(proc_path.join("cmdline"))
            .map(|s| s.replace('\0', " ").trim().to_string())
            .unwrap_or_default();

        // Parse stat
        let stat_start = stat.find('(')? + 1;
        let stat_end = stat.rfind(')')?;
        let name = stat[stat_start..stat_end].to_string();
        let stat_rest: Vec<&str> = stat[stat_end + 2..].split_whitespace().collect();

        let state = match stat_rest.first() {
            Some(&"R") => ProcessState::Running,
            Some(&"S") | Some(&"D") => ProcessState::Sleeping,
            Some(&"T") => ProcessState::Stopped,
            Some(&"Z") => ProcessState::Zombie,
            _ => ProcessState::Unknown,
        };

        let utime: u64 = stat_rest.get(11).and_then(|s| s.parse().ok()).unwrap_or(0);
        let stime: u64 = stat_rest.get(12).and_then(|s| s.parse().ok()).unwrap_or(0);
        let threads: u32 = stat_rest.get(17).and_then(|s| s.parse().ok()).unwrap_or(1);
        let rss_pages: u64 = stat_rest.get(21).and_then(|s| s.parse().ok()).unwrap_or(0);

        let memory_bytes = rss_pages * page_size;
        let memory_percent = (memory_bytes as f32 / total_mem as f32) * 100.0;

        // Calculate CPU usage
        let total_time = utime + stime;
        let cpu_percent = if let Some((prev_utime, prev_stime)) = self.prev_proc_times.get(&pid) {
            let delta = total_time.saturating_sub(prev_utime + prev_stime);
            (delta as f32 / ticks as f32) * 100.0
        } else {
            0.0
        };
        self.prev_proc_times.insert(pid, (utime, stime));

        // Get user from status
        let user = status
            .lines()
            .find(|l| l.starts_with("Uid:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|uid| Self::uid_to_username(uid.parse().ok()?))
            .unwrap_or_else(|| "unknown".to_string());

        let command = if cmdline.is_empty() { format!("[{}]", name) } else { cmdline };

        Some(ProcessInfo {
            pid,
            name,
            command,
            user,
            cpu_percent,
            memory_percent,
            memory_bytes,
            state,
            threads,
        })
    }

    fn uid_to_username(uid: u32) -> Option<String> {
        let passwd = fs::read_to_string("/etc/passwd").ok()?;
        for line in passwd.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(u) = parts[2].parse::<u32>() {
                    if u == uid {
                        return Some(parts[0].to_string());
                    }
                }
            }
        }
        Some(uid.to_string())
    }
}
