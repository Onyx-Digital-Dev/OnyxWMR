//! osv-control - Control utility for osvwm
//!
//! Provides commands for managing:
//! - Wallpapers and display settings
//! - Network (WiFi, VPN) via NetworkManager
//! - System info (user, hostname, avatar)
//! - Power controls (logout, reboot, shutdown)
//! - Status applets (battery, audio, etc.)

pub mod backend;

use anyhow::{Context, Result};
use backend::{applets, display, network, power, system};
use clap::{Parser, Subcommand};
use osv_ipc::socket::Socket;
use osv_ipc::{Action, Request, Response, WallpaperMode};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "osv-control")]
#[command(author, version, about = "Control utility for osvwm")]
struct Cli {
    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set the desktop wallpaper
    Wallpaper {
        #[command(subcommand)]
        action: WallpaperAction,
    },
    /// Query compositor information
    Query {
        #[command(subcommand)]
        what: QueryCommand,
    },
    /// Network management (WiFi, VPN)
    Network {
        #[command(subcommand)]
        action: NetworkAction,
    },
    /// Display/monitor management
    Display {
        #[command(subcommand)]
        action: DisplayAction,
    },
    /// System information
    System {
        #[command(subcommand)]
        action: SystemAction,
    },
    /// Power controls
    Power {
        #[command(subcommand)]
        action: PowerAction,
    },
    /// Status applets (battery, audio, etc.)
    Status,
}

#[derive(Subcommand)]
enum WallpaperAction {
    /// Set wallpaper from an image file
    Set {
        /// Path to the wallpaper image (PNG or JPEG)
        path: PathBuf,

        /// How to display the wallpaper
        #[arg(short, long, default_value = "fill")]
        mode: WallpaperModeArg,

        /// Specific output to set wallpaper on (default: all outputs)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Clear the wallpaper (revert to solid color)
    Clear {
        /// Specific output to clear wallpaper on (default: all outputs)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List available wallpapers
    List,
}

#[derive(Subcommand)]
enum QueryCommand {
    /// Get information about connected outputs
    Outputs,
    /// Get information about workspaces
    Workspaces,
    /// Get information about open windows
    Windows,
    /// Get the compositor version
    Version,
}

#[derive(Subcommand)]
enum NetworkAction {
    /// Show network status
    Status,
    /// WiFi operations
    Wifi {
        #[command(subcommand)]
        action: WifiAction,
    },
    /// VPN operations
    Vpn {
        #[command(subcommand)]
        action: VpnAction,
    },
}

#[derive(Subcommand)]
enum WifiAction {
    /// Scan for available networks
    Scan,
    /// Connect to a network
    Connect {
        /// SSID of the network
        ssid: String,
        /// Password (if required)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Disconnect from current network
    Disconnect,
    /// Forget a saved network
    Forget {
        /// SSID of the network
        ssid: String,
    },
    /// Enable WiFi
    Enable,
    /// Disable WiFi
    Disable,
}

#[derive(Subcommand)]
enum VpnAction {
    /// List VPN connections
    List,
    /// Connect to a VPN
    Connect {
        /// VPN connection name
        name: String,
    },
    /// Disconnect from a VPN
    Disconnect {
        /// VPN connection name
        name: String,
    },
    /// Import OpenVPN configuration
    ImportOpenvpn {
        /// Path to .ovpn file
        path: PathBuf,
        /// Optional connection name
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Import WireGuard configuration
    ImportWireguard {
        /// Path to .conf file
        path: PathBuf,
        /// Optional connection name
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Delete a VPN connection
    Delete {
        /// VPN connection name
        name: String,
    },
}

#[derive(Subcommand)]
enum DisplayAction {
    /// List connected displays
    List,
    /// Set display mode (resolution/refresh)
    Mode {
        /// Output name (e.g., DP-1)
        output: String,
        /// Resolution and refresh (e.g., 1920x1080@60)
        mode: String,
    },
    /// Set display position
    Position {
        /// Output name
        output: String,
        /// X position
        x: i32,
        /// Y position
        y: i32,
    },
    /// Set display rotation
    Rotate {
        /// Output name
        output: String,
        /// Rotation (normal, 90, 180, 270)
        rotation: String,
    },
    /// Set display scale
    Scale {
        /// Output name
        output: String,
        /// Scale factor (e.g., 1.5)
        scale: f32,
    },
    /// Enable/disable a display
    Toggle {
        /// Output name
        output: String,
        /// Enable or disable
        #[arg(long)]
        enable: bool,
    },
}

#[derive(Subcommand)]
enum SystemAction {
    /// Show user information
    User,
    /// Show system information
    Info,
    /// Show/set avatar
    Avatar {
        /// Set avatar from image path
        #[arg(short, long)]
        set: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum PowerAction {
    /// Log out of the session
    Logout,
    /// Lock the screen
    Lock,
    /// Suspend the system
    Suspend,
    /// Hibernate the system
    Hibernate,
    /// Reboot the system
    Reboot,
    /// Power off the system
    Poweroff,
    /// Show available power capabilities
    Capabilities,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum WallpaperModeArg {
    /// Scale to fill the screen, cropping if necessary
    Fill,
    /// Scale to fit within the screen, with letterboxing
    Fit,
    /// Stretch to fill the screen exactly
    Stretch,
    /// Center the image without scaling
    Center,
    /// Tile the image across the screen
    Tile,
}

impl From<WallpaperModeArg> for WallpaperMode {
    fn from(arg: WallpaperModeArg) -> Self {
        match arg {
            WallpaperModeArg::Fill => WallpaperMode::Fill,
            WallpaperModeArg::Fit => WallpaperMode::Fit,
            WallpaperModeArg::Stretch => WallpaperMode::Stretch,
            WallpaperModeArg::Center => WallpaperMode::Center,
            WallpaperModeArg::Tile => WallpaperMode::Tile,
        }
    }
}

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Wallpaper { action } => handle_wallpaper(action, cli.json),
        Commands::Query { what } => handle_query(what),
        Commands::Network { action } => handle_network(action, cli.json),
        Commands::Display { action } => handle_display(action, cli.json),
        Commands::System { action } => handle_system(action, cli.json),
        Commands::Power { action } => handle_power(action, cli.json),
        Commands::Status => handle_status(cli.json),
    }
}

fn handle_wallpaper(action: WallpaperAction, json: bool) -> Result<()> {
    match action {
        WallpaperAction::List => {
            let wallpapers = display::DisplayBackend::list_wallpapers()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&wallpapers)?);
            } else {
                for path in wallpapers {
                    println!("{}", path.display());
                }
            }
            Ok(())
        }
        WallpaperAction::Set { path, mode, output } => {
            let mut socket = Socket::connect().context("Failed to connect to osvwm")?;

            if !path.exists() {
                anyhow::bail!("Wallpaper file not found: {}", path.display());
            }

            let abs_path = path
                .canonicalize()
                .context("Failed to resolve wallpaper path")?;

            let path_str = abs_path
                .to_str()
                .context("Wallpaper path contains invalid UTF-8")?
                .to_string();

            info!("Setting wallpaper: {}", path_str);

            let request = Request::Action(Action::SetWallpaper {
                path: path_str,
                mode: mode.into(),
                output,
            });

            let reply = socket.send(request).context("Failed to send request")?;
            handle_ipc_reply(reply, json)
        }
        WallpaperAction::Clear { output } => {
            let mut socket = Socket::connect().context("Failed to connect to osvwm")?;
            info!("Clearing wallpaper");
            let request = Request::Action(Action::ClearWallpaper { output });
            let reply = socket.send(request).context("Failed to send request")?;
            handle_ipc_reply(reply, json)
        }
    }
}

fn handle_query(what: QueryCommand) -> Result<()> {
    let mut socket = Socket::connect().context("Failed to connect to osvwm")?;

    let request = match what {
        QueryCommand::Outputs => Request::Outputs,
        QueryCommand::Workspaces => Request::Workspaces,
        QueryCommand::Windows => Request::Windows,
        QueryCommand::Version => Request::Version,
    };

    let reply = socket.send(request).context("Failed to send request")?;

    match reply {
        Ok(response) => {
            let json = serde_json::to_string_pretty(&response)?;
            println!("{}", json);
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Query failed: {}", e);
        }
    }
}

fn handle_network(action: NetworkAction, json: bool) -> Result<()> {
    match action {
        NetworkAction::Status => {
            let status = network::NetworkBackend::status()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!("WiFi: {}", if status.wifi_enabled { "enabled" } else { "disabled" });
                println!("Connection: {:?}", status.connection_type);
                if let Some(wifi) = status.connected_wifi {
                    println!("Connected WiFi: {}", wifi);
                }
                if let Some(vpn) = status.active_vpn {
                    println!("Active VPN: {}", vpn);
                }
            }
            Ok(())
        }
        NetworkAction::Wifi { action } => handle_wifi(action, json),
        NetworkAction::Vpn { action } => handle_vpn(action, json),
    }
}

fn handle_wifi(action: WifiAction, json: bool) -> Result<()> {
    match action {
        WifiAction::Scan => {
            let networks = network::NetworkBackend::scan_wifi()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&networks)?);
            } else {
                println!("{:<32} {:>6} {:<15} {}", "SSID", "SIGNAL", "SECURITY", "CONNECTED");
                println!("{}", "-".repeat(65));
                for net in networks {
                    println!(
                        "{:<32} {:>5}% {:<15} {}",
                        net.ssid,
                        net.signal,
                        net.security,
                        if net.connected { "*" } else { "" }
                    );
                }
            }
            Ok(())
        }
        WifiAction::Connect { ssid, password } => {
            network::NetworkBackend::connect_wifi(&ssid, password.as_deref())?;
            println!("Connected to '{}'", ssid);
            Ok(())
        }
        WifiAction::Disconnect => {
            network::NetworkBackend::disconnect_wifi()?;
            println!("Disconnected from WiFi");
            Ok(())
        }
        WifiAction::Forget { ssid } => {
            network::NetworkBackend::forget_wifi(&ssid)?;
            println!("Forgot network '{}'", ssid);
            Ok(())
        }
        WifiAction::Enable => {
            network::NetworkBackend::set_wifi_enabled(true)?;
            println!("WiFi enabled");
            Ok(())
        }
        WifiAction::Disable => {
            network::NetworkBackend::set_wifi_enabled(false)?;
            println!("WiFi disabled");
            Ok(())
        }
    }
}

fn handle_vpn(action: VpnAction, json: bool) -> Result<()> {
    match action {
        VpnAction::List => {
            let vpns = network::NetworkBackend::list_vpns()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&vpns)?);
            } else {
                println!("{:<30} {:<15} {}", "NAME", "TYPE", "ACTIVE");
                println!("{}", "-".repeat(50));
                for vpn in vpns {
                    println!(
                        "{:<30} {:<15} {}",
                        vpn.name,
                        vpn.vpn_type,
                        if vpn.active { "*" } else { "" }
                    );
                }
            }
            Ok(())
        }
        VpnAction::Connect { name } => {
            network::NetworkBackend::connect_vpn(&name)?;
            println!("Connected to VPN '{}'", name);
            Ok(())
        }
        VpnAction::Disconnect { name } => {
            network::NetworkBackend::disconnect_vpn(&name)?;
            println!("Disconnected from VPN '{}'", name);
            Ok(())
        }
        VpnAction::ImportOpenvpn { path, name } => {
            let imported = network::NetworkBackend::import_openvpn(&path, name.as_deref())?;
            println!("Imported OpenVPN connection: {}", imported);
            Ok(())
        }
        VpnAction::ImportWireguard { path, name } => {
            let imported = network::NetworkBackend::import_wireguard(&path, name.as_deref())?;
            println!("Imported WireGuard connection: {}", imported);
            Ok(())
        }
        VpnAction::Delete { name } => {
            network::NetworkBackend::delete_vpn(&name)?;
            println!("Deleted VPN '{}'", name);
            Ok(())
        }
    }
}

fn handle_display(action: DisplayAction, json: bool) -> Result<()> {
    match action {
        DisplayAction::List => {
            let displays = display::DisplayBackend::list_displays()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&displays)?);
            } else {
                for disp in displays {
                    println!("{} ({} {})", disp.name, disp.make, disp.model);
                    if let Some(mode) = disp.current_mode {
                        println!("  Mode: {}x{}@{:.0}Hz", mode.width, mode.height, mode.refresh);
                    }
                    println!("  Position: {},{}", disp.position.x, disp.position.y);
                    println!("  Scale: {}", disp.scale);
                    println!("  Enabled: {}", disp.enabled);
                    println!();
                }
            }
            Ok(())
        }
        DisplayAction::Mode { output, mode } => {
            // Parse mode string like "1920x1080@60"
            let parts: Vec<&str> = mode.split('@').collect();
            let res_parts: Vec<u32> = parts[0]
                .split('x')
                .filter_map(|s| s.parse().ok())
                .collect();

            if res_parts.len() != 2 {
                anyhow::bail!("Invalid mode format. Use: WIDTHxHEIGHT@REFRESH (e.g., 1920x1080@60)");
            }

            let refresh = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(60.0);

            let mode = display::DisplayMode {
                width: res_parts[0],
                height: res_parts[1],
                refresh,
                preferred: false,
            };

            display::DisplayBackend::set_mode(&output, &mode)?;
            println!("Set {} to {}x{}@{}Hz", output, mode.width, mode.height, mode.refresh);
            Ok(())
        }
        DisplayAction::Position { output, x, y } => {
            display::DisplayBackend::set_position(&output, x, y)?;
            println!("Set {} position to {},{}", output, x, y);
            Ok(())
        }
        DisplayAction::Rotate { output, rotation } => {
            let transform = display::Transform::from_str(&rotation);
            display::DisplayBackend::set_transform(&output, transform)?;
            println!("Set {} rotation to {:?}", output, transform);
            Ok(())
        }
        DisplayAction::Scale { output, scale } => {
            display::DisplayBackend::set_scale(&output, scale)?;
            println!("Set {} scale to {}", output, scale);
            Ok(())
        }
        DisplayAction::Toggle { output, enable } => {
            display::DisplayBackend::set_enabled(&output, enable)?;
            println!("{} {}", if enable { "Enabled" } else { "Disabled" }, output);
            Ok(())
        }
    }
}

fn handle_system(action: SystemAction, json: bool) -> Result<()> {
    match action {
        SystemAction::User => {
            let user = system::SystemBackend::user_info()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&user)?);
            } else {
                println!("Username: {}", user.username);
                if let Some(name) = user.full_name {
                    println!("Full name: {}", name);
                }
                println!("Home: {}", user.home_dir.display());
                println!("UID: {}", user.uid);
                println!("Shell: {}", user.shell.display());
            }
            Ok(())
        }
        SystemAction::Info => {
            let info = system::SystemBackend::system_info()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("Hostname: {}", info.hostname);
                if let Some(pretty) = info.pretty_hostname {
                    println!("Pretty hostname: {}", pretty);
                }
                println!("OS: {}", info.os_name);
                if let Some(version) = info.os_version {
                    println!("Version: {}", version);
                }
                println!("Kernel: {}", info.kernel_version);
                println!("Architecture: {}", info.architecture);
            }
            Ok(())
        }
        SystemAction::Avatar { set } => {
            if let Some(path) = set {
                system::SystemBackend::set_avatar(&path)?;
                println!("Avatar set to: {}", path.display());
            } else {
                let config = system::SystemBackend::avatar_config()?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&config)?);
                } else {
                    if let Some(path) = config.path {
                        println!("Current avatar: {}", path.display());
                    } else {
                        println!("No avatar set (initials: {})", system::SystemBackend::user_initials());
                    }
                    println!("\nAvailable presets:");
                    for preset in config.presets {
                        println!("  {}", preset.display());
                    }
                }
            }
            Ok(())
        }
    }
}

fn handle_power(action: PowerAction, json: bool) -> Result<()> {
    match action {
        PowerAction::Capabilities => {
            let caps = power::PowerBackend::capabilities()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&caps)?);
            } else {
                println!("Suspend: {}", if caps.can_suspend { "available" } else { "not available" });
                println!("Hibernate: {}", if caps.can_hibernate { "available" } else { "not available" });
                println!("Hybrid sleep: {}", if caps.can_hybrid_sleep { "available" } else { "not available" });
                println!("Reboot: {}", if caps.can_reboot { "available" } else { "not available" });
                println!("Power off: {}", if caps.can_power_off { "available" } else { "not available" });
            }
            Ok(())
        }
        PowerAction::Logout => {
            power::PowerBackend::execute(power::PowerAction::Logout)?;
            Ok(())
        }
        PowerAction::Lock => {
            power::PowerBackend::execute(power::PowerAction::Lock)?;
            Ok(())
        }
        PowerAction::Suspend => {
            power::PowerBackend::execute(power::PowerAction::Suspend)?;
            Ok(())
        }
        PowerAction::Hibernate => {
            power::PowerBackend::execute(power::PowerAction::Hibernate)?;
            Ok(())
        }
        PowerAction::Reboot => {
            power::PowerBackend::execute(power::PowerAction::Reboot)?;
            Ok(())
        }
        PowerAction::Poweroff => {
            power::PowerBackend::execute(power::PowerAction::PowerOff)?;
            Ok(())
        }
    }
}

fn handle_status(json: bool) -> Result<()> {
    let status = applets::AppletsBackend::status();
    if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        // Battery
        if let Some(battery) = status.battery {
            println!("Battery: {}% ({:?})", battery.percentage, battery.state);
            if let Some(time) = battery.time_remaining {
                let hours = time / 3600;
                let minutes = (time % 3600) / 60;
                println!("  Time remaining: {}:{:02}", hours, minutes);
            }
        }

        // Audio
        println!(
            "Audio: {}%{}",
            status.audio.volume,
            if status.audio.muted { " (muted)" } else { "" }
        );

        // Network
        println!(
            "Network: {:?}{}",
            status.network.connection_type,
            if status.network.vpn_active { " + VPN" } else { "" }
        );
        if let Some(name) = status.network.network_name {
            println!("  Connected to: {}", name);
        }

        // Bluetooth
        if status.bluetooth.available {
            println!(
                "Bluetooth: {}",
                if status.bluetooth.powered {
                    format!("on ({} devices)", status.bluetooth.connected_devices)
                } else {
                    "off".to_string()
                }
            );
        }

        // Resources
        println!(
            "CPU: {:.1}% | Memory: {:.1}%",
            status.resources.cpu_percent, status.resources.memory_percent
        );

        // Date/time
        println!("{} | {}", status.datetime.date_formatted, status.datetime.time_formatted);
    }
    Ok(())
}

fn handle_ipc_reply(reply: std::result::Result<Response, String>, json: bool) -> Result<()> {
    match reply {
        Ok(Response::Handled) => {
            if !json {
                println!("OK");
            }
            Ok(())
        }
        Ok(other) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&other)?);
            } else {
                println!("{:?}", other);
            }
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Request failed: {}", e);
        }
    }
}
