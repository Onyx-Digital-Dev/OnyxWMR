//! Power management backend
//!
//! Provides logout, reboot, shutdown, suspend, and hibernate controls.
//! Uses systemd/logind for power management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Power action type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PowerAction {
    /// Log out the current session
    Logout,
    /// Lock the screen
    Lock,
    /// Suspend to RAM
    Suspend,
    /// Hibernate to disk
    Hibernate,
    /// Hybrid sleep (suspend + hibernate)
    HybridSleep,
    /// Reboot the system
    Reboot,
    /// Power off the system
    PowerOff,
}

/// Power capabilities - what actions are available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerCapabilities {
    pub can_suspend: bool,
    pub can_hibernate: bool,
    pub can_hybrid_sleep: bool,
    pub can_reboot: bool,
    pub can_power_off: bool,
}

/// Power backend
pub struct PowerBackend;

impl PowerBackend {
    /// Get available power capabilities
    pub fn capabilities() -> Result<PowerCapabilities> {
        Ok(PowerCapabilities {
            can_suspend: Self::check_capability("CanSuspend"),
            can_hibernate: Self::check_capability("CanHibernate"),
            can_hybrid_sleep: Self::check_capability("CanHybridSleep"),
            can_reboot: Self::check_capability("CanReboot"),
            can_power_off: Self::check_capability("CanPowerOff"),
        })
    }

    /// Check if a specific power capability is available via loginctl
    fn check_capability(capability: &str) -> bool {
        let output = Command::new("loginctl")
            .args(["show-session", "-p", capability])
            .output()
            .ok();

        if let Some(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("=yes");
        }

        // Fallback: assume available if known capability
        matches!(
            capability,
            "CanSuspend" | "CanHibernate" | "CanHybridSleep" | "CanReboot" | "CanPowerOff"
        )
    }

    /// Execute a power action
    pub fn execute(action: PowerAction) -> Result<()> {
        match action {
            PowerAction::Logout => Self::logout(),
            PowerAction::Lock => Self::lock(),
            PowerAction::Suspend => Self::systemctl_action("suspend"),
            PowerAction::Hibernate => Self::systemctl_action("hibernate"),
            PowerAction::HybridSleep => Self::systemctl_action("hybrid-sleep"),
            PowerAction::Reboot => Self::systemctl_action("reboot"),
            PowerAction::PowerOff => Self::systemctl_action("poweroff"),
        }
    }

    /// Log out the current session
    fn logout() -> Result<()> {
        // Try multiple methods in order of preference

        // 1. Try compositor-specific logout via IPC
        // This would be handled by osv-ipc in a real implementation

        // 2. Try loginctl terminate-session
        if let Ok(session) = std::env::var("XDG_SESSION_ID") {
            let output = Command::new("loginctl")
                .args(["terminate-session", &session])
                .output()
                .context("Failed to run loginctl")?;

            if output.status.success() {
                return Ok(());
            }
        }

        // 3. Try loginctl terminate-user
        if let Ok(user) = std::env::var("USER") {
            let output = Command::new("loginctl")
                .args(["terminate-user", &user])
                .output()
                .context("Failed to run loginctl")?;

            if output.status.success() {
                return Ok(());
            }
        }

        // 4. Fallback: kill the compositor process group
        // This is a last resort
        anyhow::bail!("Could not find a way to logout. Please close the compositor manually.");
    }

    /// Lock the screen
    fn lock() -> Result<()> {
        // Try multiple methods

        // 1. Try loginctl lock-session
        if let Ok(session) = std::env::var("XDG_SESSION_ID") {
            let output = Command::new("loginctl")
                .args(["lock-session", &session])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    return Ok(());
                }
            }
        }

        // 2. Try common screen lockers
        let lockers: [&str; 3] = ["swaylock", "waylock", "gtklock"];

        for locker in &lockers {
            if Command::new("which")
                .arg(locker)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                Command::new(locker)
                    .spawn()
                    .context(format!("Failed to run {}", locker))?;

                // Screen locker started successfully
                return Ok(());
            }
        }

        anyhow::bail!("No screen locker found. Install swaylock, waylock, or gtklock.");
    }

    /// Execute a systemctl power action
    fn systemctl_action(action: &str) -> Result<()> {
        let output = Command::new("systemctl")
            .arg(action)
            .output()
            .context(format!("Failed to run systemctl {}", action))?;

        if !output.status.success() {
            // Try with pkexec for privilege escalation
            let output = Command::new("pkexec")
                .args(["systemctl", action])
                .output()
                .context("Failed to run pkexec systemctl")?;

            if !output.status.success() {
                anyhow::bail!(
                    "Failed to {}: {}",
                    action,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        Ok(())
    }

    /// Schedule a shutdown/reboot with optional delay
    pub fn schedule(action: PowerAction, delay_minutes: u32) -> Result<()> {
        let action_str = match action {
            PowerAction::Reboot => "reboot",
            PowerAction::PowerOff => "poweroff",
            _ => anyhow::bail!("Can only schedule reboot or poweroff"),
        };

        let delay = if delay_minutes == 0 {
            "now".to_string()
        } else {
            format!("+{}", delay_minutes)
        };

        let output = Command::new("shutdown")
            .args([action_str, &delay])
            .output()
            .context("Failed to schedule shutdown")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to schedule {}: {}",
                action_str,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Cancel a scheduled shutdown
    pub fn cancel_schedule() -> Result<()> {
        let output = Command::new("shutdown")
            .arg("-c")
            .output()
            .context("Failed to cancel shutdown")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to cancel shutdown: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Get the current session inhibitors (things preventing sleep/shutdown)
    pub fn list_inhibitors() -> Result<Vec<Inhibitor>> {
        let output = Command::new("systemd-inhibit")
            .arg("--list")
            .output()
            .context("Failed to list inhibitors")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut inhibitors = Vec::new();

        // Parse the output (skip header line)
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                inhibitors.push(Inhibitor {
                    what: parts[0].to_string(),
                    who: parts[1].to_string(),
                    why: parts[2..parts.len() - 2].join(" "),
                    mode: parts[parts.len() - 1].to_string(),
                });
            }
        }

        Ok(inhibitors)
    }
}

/// System inhibitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inhibitor {
    /// What is being inhibited (shutdown, sleep, etc.)
    pub what: String,
    /// Who is inhibiting
    pub who: String,
    /// Why it's being inhibited
    pub why: String,
    /// Inhibit mode (block or delay)
    pub mode: String,
}

