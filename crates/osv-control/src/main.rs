//! osv-control - Control utility for osvwm
//!
//! Provides commands for managing wallpapers, settings, and other
//! compositor configuration through the IPC interface.

use anyhow::{Context, Result};
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
        Commands::Wallpaper { action } => handle_wallpaper(action),
        Commands::Query { what } => handle_query(what),
    }
}

fn handle_wallpaper(action: WallpaperAction) -> Result<()> {
    let mut socket = Socket::connect().context("Failed to connect to osvwm")?;

    let request = match action {
        WallpaperAction::Set { path, mode, output } => {
            // Validate path exists
            if !path.exists() {
                anyhow::bail!("Wallpaper file not found: {}", path.display());
            }

            // Convert to absolute path
            let abs_path = path
                .canonicalize()
                .context("Failed to resolve wallpaper path")?;

            let path_str = abs_path
                .to_str()
                .context("Wallpaper path contains invalid UTF-8")?
                .to_string();

            info!("Setting wallpaper: {}", path_str);

            Request::Action(Action::SetWallpaper {
                path: path_str,
                mode: mode.into(),
                output,
            })
        }
        WallpaperAction::Clear { output } => {
            info!("Clearing wallpaper");
            Request::Action(Action::ClearWallpaper { output })
        }
    };

    let reply = socket.send(request).context("Failed to send request")?;

    match reply {
        Ok(Response::Handled) => {
            println!("OK");
            Ok(())
        }
        Ok(other) => {
            println!("Unexpected response: {:?}", other);
            Ok(())
        }
        Err(e) => {
            anyhow::bail!("Request failed: {}", e);
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
