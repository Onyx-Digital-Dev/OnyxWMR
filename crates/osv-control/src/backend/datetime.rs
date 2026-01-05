//! Date and Time settings backend
//!
//! Provides date/time display, timezone management, and NTP synchronization control.
//! Uses timedatectl (systemd) when available, with fallback to direct methods.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Time format preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimeFormat {
    /// 12-hour format with AM/PM
    TwelveHour,
    /// 24-hour format
    #[default]
    TwentyFourHour,
}

/// Date format preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DateFormat {
    /// Year-Month-Day (2024-01-15)
    #[default]
    YearMonthDay,
    /// Month/Day/Year (01/15/2024)
    MonthDayYear,
    /// Day.Month.Year (15.01.2024)
    DayMonthYear,
}

/// NTP synchronization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtpStatus {
    /// Whether NTP is enabled
    pub enabled: bool,
    /// Whether time is currently synchronized
    pub synchronized: bool,
    /// NTP server in use (if available)
    pub server: Option<String>,
    /// Last sync time as Unix timestamp
    pub last_sync: Option<u64>,
}

/// Timezone information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimezoneInfo {
    /// Timezone identifier (e.g., "America/New_York")
    pub identifier: String,
    /// Display name (e.g., "Eastern Time")
    pub display_name: String,
    /// UTC offset in seconds
    pub utc_offset: i32,
    /// UTC offset string (e.g., "UTC-05:00")
    pub utc_offset_str: String,
}

/// Current date and time information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateTimeInfo {
    /// Unix timestamp
    pub timestamp: u64,
    /// Year
    pub year: i32,
    /// Month (1-12)
    pub month: u8,
    /// Day of month (1-31)
    pub day: u8,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
    /// Day of week (0 = Sunday, 6 = Saturday)
    pub weekday: u8,
    /// Formatted date string
    pub date_str: String,
    /// Formatted time string (24h)
    pub time_str_24h: String,
    /// Formatted time string (12h with AM/PM)
    pub time_str_12h: String,
    /// Current timezone
    pub timezone: TimezoneInfo,
}

/// Date and time backend
pub struct DateTimeBackend {
    /// Preferred time format
    pub time_format: TimeFormat,
    /// Preferred date format
    pub date_format: DateFormat,
}

impl Default for DateTimeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DateTimeBackend {
    /// Create a new datetime backend
    pub fn new() -> Self {
        Self {
            time_format: TimeFormat::TwentyFourHour,
            date_format: DateFormat::YearMonthDay,
        }
    }

    /// Get current date and time information
    pub fn get_datetime(&self) -> Result<DateTimeInfo> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get system time")?;
        let timestamp = now.as_secs();

        // Parse timestamp into components
        // Days since epoch
        let days = (timestamp / 86400) as i64;
        let remaining_secs = timestamp % 86400;

        let hour = (remaining_secs / 3600) as u8;
        let minute = ((remaining_secs % 3600) / 60) as u8;
        let second = (remaining_secs % 60) as u8;

        // Calculate date from days since epoch (Jan 1, 1970)
        let (year, month, day) = days_to_ymd(days);

        // Calculate day of week (Jan 1, 1970 was Thursday = 4)
        let weekday = ((days + 4) % 7) as u8;

        let timezone = self.get_timezone()?;

        // Apply timezone offset for local time
        let local_timestamp = (timestamp as i64 + timezone.utc_offset as i64) as u64;
        let local_remaining = local_timestamp % 86400;
        let local_hour = (local_remaining / 3600) as u8;
        let local_minute = ((local_remaining % 3600) / 60) as u8;
        let local_second = (local_remaining % 60) as u8;

        let local_days = (local_timestamp / 86400) as i64;
        let (local_year, local_month, local_day) = days_to_ymd(local_days);

        // Format strings
        let date_str = match self.date_format {
            DateFormat::YearMonthDay => {
                format!("{:04}-{:02}-{:02}", local_year, local_month, local_day)
            }
            DateFormat::MonthDayYear => {
                format!("{:02}/{:02}/{:04}", local_month, local_day, local_year)
            }
            DateFormat::DayMonthYear => {
                format!("{:02}.{:02}.{:04}", local_day, local_month, local_year)
            }
        };

        let time_str_24h = format!("{:02}:{:02}:{:02}", local_hour, local_minute, local_second);

        let (hour_12, ampm) = if local_hour == 0 {
            (12, "AM")
        } else if local_hour < 12 {
            (local_hour, "AM")
        } else if local_hour == 12 {
            (12, "PM")
        } else {
            (local_hour - 12, "PM")
        };
        let time_str_12h = format!("{:02}:{:02}:{:02} {}", hour_12, local_minute, local_second, ampm);

        Ok(DateTimeInfo {
            timestamp,
            year: local_year,
            month: local_month,
            day: local_day,
            hour: local_hour,
            minute: local_minute,
            second: local_second,
            weekday,
            date_str,
            time_str_24h,
            time_str_12h,
            timezone,
        })
    }

    /// Get current timezone
    pub fn get_timezone(&self) -> Result<TimezoneInfo> {
        // Try timedatectl first
        if let Ok(output) = Command::new("timedatectl")
            .arg("show")
            .arg("--property=Timezone")
            .arg("--value")
            .output()
        {
            if output.status.success() {
                let tz = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !tz.is_empty() {
                    return Ok(self.parse_timezone(&tz));
                }
            }
        }

        // Fallback: read /etc/timezone
        if let Ok(tz) = std::fs::read_to_string("/etc/timezone") {
            let tz = tz.trim().to_string();
            if !tz.is_empty() {
                return Ok(self.parse_timezone(&tz));
            }
        }

        // Fallback: read /etc/localtime symlink
        if let Ok(link) = std::fs::read_link("/etc/localtime") {
            let path = link.to_string_lossy();
            if let Some(tz) = path.strip_prefix("/usr/share/zoneinfo/") {
                return Ok(self.parse_timezone(tz));
            }
        }

        // Default to UTC
        Ok(TimezoneInfo {
            identifier: "UTC".to_string(),
            display_name: "Coordinated Universal Time".to_string(),
            utc_offset: 0,
            utc_offset_str: "UTC+00:00".to_string(),
        })
    }

    /// Parse timezone identifier into TimezoneInfo
    fn parse_timezone(&self, identifier: &str) -> TimezoneInfo {
        // Get offset from date command
        let offset = Command::new("date")
            .arg("+%z")
            .env("TZ", identifier)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    parse_offset_string(&s)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let offset_hours = offset / 3600;
        let offset_minutes = (offset.abs() % 3600) / 60;
        let sign = if offset >= 0 { "+" } else { "-" };
        let utc_offset_str = format!("UTC{}{:02}:{:02}", sign, offset_hours.abs(), offset_minutes);

        // Generate display name from identifier
        let display_name = identifier
            .split('/')
            .last()
            .unwrap_or(identifier)
            .replace('_', " ");

        TimezoneInfo {
            identifier: identifier.to_string(),
            display_name,
            utc_offset: offset,
            utc_offset_str,
        }
    }

    /// Set timezone (requires root/polkit)
    pub fn set_timezone(&self, timezone: &str) -> Result<()> {
        let status = Command::new("timedatectl")
            .args(["set-timezone", timezone])
            .status()
            .context("Failed to run timedatectl")?;

        if !status.success() {
            anyhow::bail!("Failed to set timezone to {}", timezone);
        }

        Ok(())
    }

    /// Get NTP synchronization status
    pub fn get_ntp_status(&self) -> Result<NtpStatus> {
        // Try timedatectl
        let output = Command::new("timedatectl")
            .arg("show")
            .output()
            .context("Failed to run timedatectl")?;

        if !output.status.success() {
            return Ok(NtpStatus {
                enabled: false,
                synchronized: false,
                server: None,
                last_sync: None,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut enabled = false;
        let mut synchronized = false;

        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "NTP" => enabled = value == "yes",
                    "NTPSynchronized" => synchronized = value == "yes",
                    _ => {}
                }
            }
        }

        // Try to get NTP server from systemd-timesyncd
        let server = std::fs::read_to_string("/run/systemd/timesync/synchronized")
            .ok()
            .and_then(|_| {
                Command::new("systemctl")
                    .args(["status", "systemd-timesyncd"])
                    .output()
                    .ok()
            })
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                // Parse server from status output
                s.lines()
                    .find(|l| l.contains("Server:"))
                    .map(|l| l.split_whitespace().last().unwrap_or("").to_string())
            });

        Ok(NtpStatus {
            enabled,
            synchronized,
            server,
            last_sync: None,
        })
    }

    /// Enable or disable NTP synchronization
    pub fn set_ntp(&self, enabled: bool) -> Result<()> {
        let value = if enabled { "true" } else { "false" };
        let status = Command::new("timedatectl")
            .args(["set-ntp", value])
            .status()
            .context("Failed to run timedatectl")?;

        if !status.success() {
            anyhow::bail!("Failed to {} NTP", if enabled { "enable" } else { "disable" });
        }

        Ok(())
    }

    /// Set manual time (requires NTP disabled)
    pub fn set_time(&self, hour: u8, minute: u8, second: u8) -> Result<()> {
        let time_str = format!("{:02}:{:02}:{:02}", hour, minute, second);
        let status = Command::new("timedatectl")
            .args(["set-time", &time_str])
            .status()
            .context("Failed to run timedatectl")?;

        if !status.success() {
            anyhow::bail!("Failed to set time to {}", time_str);
        }

        Ok(())
    }

    /// Set manual date (requires NTP disabled)
    pub fn set_date(&self, year: i32, month: u8, day: u8) -> Result<()> {
        let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
        let status = Command::new("timedatectl")
            .args(["set-time", &date_str])
            .status()
            .context("Failed to run timedatectl")?;

        if !status.success() {
            anyhow::bail!("Failed to set date to {}", date_str);
        }

        Ok(())
    }

    /// Get list of available timezones
    pub fn list_timezones(&self) -> Result<Vec<String>> {
        let output = Command::new("timedatectl")
            .args(["list-timezones"])
            .output()
            .context("Failed to run timedatectl")?;

        if !output.status.success() {
            anyhow::bail!("Failed to list timezones");
        }

        let timezones: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(timezones)
    }

    /// Get formatted time string based on current preferences
    pub fn get_formatted_time(&self) -> Result<String> {
        let info = self.get_datetime()?;
        Ok(match self.time_format {
            TimeFormat::TwelveHour => info.time_str_12h,
            TimeFormat::TwentyFourHour => info.time_str_24h,
        })
    }

    /// Get formatted date string based on current preferences
    pub fn get_formatted_date(&self) -> Result<String> {
        let info = self.get_datetime()?;
        Ok(info.date_str)
    }

    /// Get day of week name
    pub fn get_weekday_name(weekday: u8) -> &'static str {
        match weekday {
            0 => "Sunday",
            1 => "Monday",
            2 => "Tuesday",
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            _ => "Unknown",
        }
    }

    /// Get month name
    pub fn get_month_name(month: u8) -> &'static str {
        match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        }
    }
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_ymd(days: i64) -> (i32, u8, u8) {
    // Algorithm from Howard Hinnant
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    (year as i32, m as u8, d as u8)
}

/// Parse offset string like "+0500" or "-0800" to seconds
fn parse_offset_string(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.len() < 5 {
        return None;
    }

    let sign = match s.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };

    let hours: i32 = s[1..3].parse().ok()?;
    let minutes: i32 = s[3..5].parse().ok()?;

    Some(sign * (hours * 3600 + minutes * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_to_ymd() {
        // Jan 1, 1970
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        // Dec 31, 1969
        assert_eq!(days_to_ymd(-1), (1969, 12, 31));
        // Jan 1, 2000
        assert_eq!(days_to_ymd(10957), (2000, 1, 1));
        // Jan 1, 2024
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
    }

    #[test]
    fn test_parse_offset_string() {
        assert_eq!(parse_offset_string("+0000"), Some(0));
        assert_eq!(parse_offset_string("+0500"), Some(5 * 3600));
        assert_eq!(parse_offset_string("-0800"), Some(-8 * 3600));
        assert_eq!(parse_offset_string("+0530"), Some(5 * 3600 + 30 * 60));
    }

    #[test]
    fn test_weekday_names() {
        assert_eq!(DateTimeBackend::get_weekday_name(0), "Sunday");
        assert_eq!(DateTimeBackend::get_weekday_name(1), "Monday");
        assert_eq!(DateTimeBackend::get_weekday_name(6), "Saturday");
    }

    #[test]
    fn test_month_names() {
        assert_eq!(DateTimeBackend::get_month_name(1), "January");
        assert_eq!(DateTimeBackend::get_month_name(12), "December");
    }
}
