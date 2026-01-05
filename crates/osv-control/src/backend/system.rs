//! System information backend
//!
//! Provides user info, hostname, and avatar management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Username
    pub username: String,
    /// Full name (from GECOS field)
    pub full_name: Option<String>,
    /// Home directory
    pub home_dir: PathBuf,
    /// User ID
    pub uid: u32,
    /// Primary group ID
    pub gid: u32,
    /// Shell
    pub shell: PathBuf,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Hostname
    pub hostname: String,
    /// Pretty hostname (if set)
    pub pretty_hostname: Option<String>,
    /// OS name
    pub os_name: String,
    /// OS version
    pub os_version: Option<String>,
    /// Kernel version
    pub kernel_version: String,
    /// Architecture
    pub architecture: String,
}

/// Avatar configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarConfig {
    /// Path to the current avatar image
    pub path: Option<PathBuf>,
    /// Available avatar presets
    pub presets: Vec<PathBuf>,
}

/// System backend
pub struct SystemBackend;

impl SystemBackend {
    /// Get current user information
    pub fn user_info() -> Result<UserInfo> {
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| whoami::username());

        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        // Get UID and GID
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };

        // Get full name from passwd
        let full_name = Self::get_gecos_name(&username);

        // Get shell
        let shell = std::env::var("SHELL")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/bin/sh"));

        Ok(UserInfo {
            username,
            full_name,
            home_dir,
            uid,
            gid,
            shell,
        })
    }

    /// Get GECOS name from /etc/passwd
    fn get_gecos_name(username: &str) -> Option<String> {
        let passwd = fs::read_to_string("/etc/passwd").ok()?;

        for line in passwd.lines() {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() >= 5 && fields[0] == username {
                let gecos = fields[4];
                // GECOS format: Full Name,Room,Phone,Other
                let name = gecos.split(',').next().unwrap_or(gecos);
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }

        None
    }

    /// Get system information
    pub fn system_info() -> Result<SystemInfo> {
        let hostname = Self::get_hostname()?;
        let pretty_hostname = Self::get_pretty_hostname();
        let os_info = Self::get_os_info();
        let kernel_version = Self::get_kernel_version()?;
        let architecture = std::env::consts::ARCH.to_string();

        Ok(SystemInfo {
            hostname,
            pretty_hostname,
            os_name: os_info.0,
            os_version: os_info.1,
            kernel_version,
            architecture,
        })
    }

    /// Get hostname
    fn get_hostname() -> Result<String> {
        // Try reading from /etc/hostname first
        if let Ok(hostname) = fs::read_to_string("/etc/hostname") {
            let hostname = hostname.trim();
            if !hostname.is_empty() {
                return Ok(hostname.to_string());
            }
        }

        // Fall back to gethostname
        Ok(whoami::fallible::hostname().unwrap_or_else(|_| "localhost".to_string()))
    }

    /// Get pretty hostname from hostnamectl
    fn get_pretty_hostname() -> Option<String> {
        // Try reading from /etc/machine-info
        if let Ok(content) = fs::read_to_string("/etc/machine-info") {
            for line in content.lines() {
                if let Some(value) = line.strip_prefix("PRETTY_HOSTNAME=") {
                    let value = value.trim_matches('"').trim_matches('\'');
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }

        None
    }

    /// Get OS name and version from /etc/os-release
    fn get_os_info() -> (String, Option<String>) {
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            let mut name = String::from("Linux");
            let mut version = None;

            for line in content.lines() {
                if let Some(value) = line.strip_prefix("NAME=") {
                    name = value.trim_matches('"').trim_matches('\'').to_string();
                } else if let Some(value) = line.strip_prefix("VERSION=") {
                    version = Some(value.trim_matches('"').trim_matches('\'').to_string());
                } else if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
                    // Use PRETTY_NAME as fallback for name
                    if name == "Linux" {
                        name = value.trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }

            return (name, version);
        }

        ("Linux".to_string(), None)
    }

    /// Get kernel version
    fn get_kernel_version() -> Result<String> {
        let output = std::process::Command::new("uname")
            .arg("-r")
            .output()
            .context("Failed to get kernel version")?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get avatar configuration
    pub fn avatar_config() -> Result<AvatarConfig> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("osv");

        let avatar_path = config_dir.join("avatar.png");
        let current_avatar = if avatar_path.exists() {
            Some(avatar_path)
        } else {
            // Check AccountsService avatar
            Self::get_accountsservice_avatar()
        };

        let presets = Self::list_avatar_presets();

        Ok(AvatarConfig {
            path: current_avatar,
            presets,
        })
    }

    /// Get avatar from AccountsService
    fn get_accountsservice_avatar() -> Option<PathBuf> {
        let uid = unsafe { libc::getuid() };
        let path = PathBuf::from(format!("/var/lib/AccountsService/icons/{}", uid));

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// List available avatar presets
    fn list_avatar_presets() -> Vec<PathBuf> {
        let mut avatars = Vec::new();

        // Common avatar directories
        let dirs = [
            "/usr/share/pixmaps/faces",
            "/usr/share/icons/hicolor/96x96/apps",
        ];

        for dir in &dirs {
            let path = Path::new(dir);
            if path.exists() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                let ext = ext.to_string_lossy().to_lowercase();
                                if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "svg") {
                                    avatars.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        avatars.sort();
        avatars
    }

    /// Set user avatar
    pub fn set_avatar(image_path: &Path) -> Result<()> {
        if !image_path.exists() {
            anyhow::bail!("Avatar image not found: {}", image_path.display());
        }

        // Create config directory
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("osv");
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        // Copy image to config
        let avatar_path = config_dir.join("avatar.png");

        // If the source is PNG, just copy it
        // For other formats, we'd need image conversion
        fs::copy(image_path, &avatar_path).context("Failed to copy avatar")?;

        Ok(())
    }

    /// Get user's initials for default avatar
    pub fn user_initials() -> String {
        let user_info = Self::user_info().ok();

        let name = user_info
            .as_ref()
            .and_then(|u| u.full_name.as_ref())
            .or_else(|| user_info.as_ref().map(|u| &u.username))
            .map(|s| s.as_str())
            .unwrap_or("U");

        // Get first letter of each word (up to 2)
        name.split_whitespace()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect::<String>()
            .to_uppercase()
    }

    /// Set hostname (requires root/sudo)
    pub fn set_hostname(hostname: &str, pretty: bool) -> Result<()> {
        let hostname_type = if pretty { "pretty" } else { "hostname" };

        let output = std::process::Command::new("hostnamectl")
            .args(["set-hostname", "--transient", hostname])
            .output()
            .context("Failed to set hostname")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to set hostname: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Also set static hostname if not pretty
        if !pretty {
            let _ = std::process::Command::new("hostnamectl")
                .args(["set-hostname", "--static", hostname])
                .output();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_initials() {
        // This is a simple test that doesn't require actual system info
        let initials = SystemBackend::user_initials();
        assert!(!initials.is_empty());
    }
}
