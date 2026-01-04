//! OSV-specific constants and types for the workspace management routine.
//!
//! This module defines the core philosophy of osvwm:
//! - 8 fixed workspaces (1-6 regular, 7-8 immersion)
//! - 1D horizontal strip layout per workspace
//! - Fixed window sizes: 1u, 2u, 3u, Frame
//! - No auto-tiling, no smart layouts

/// Total number of workspaces (fixed, not dynamic).
pub const WORKSPACE_COUNT: usize = 8;

/// First immersion workspace (0-indexed).
pub const IMMERSION_START: usize = 6;

/// Workspace names for the 8 fixed workspaces.
pub const WORKSPACE_NAMES: [&str; WORKSPACE_COUNT] = [
    "1", "2", "3", "4", "5", "6", "IMM-7", "IMM-8",
];

/// Window size unit as a fraction of screen width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowUnit {
    /// 1/3 screen width
    #[default]
    Unit1,
    /// 2/3 screen width
    Unit2,
    /// Full width (with decorations)
    Unit3,
    /// True fullscreen (no decorations)
    Frame,
}

impl WindowUnit {
    /// Get the width as a proportion of screen width (0.0 to 1.0).
    pub fn width_proportion(&self) -> f64 {
        match self {
            WindowUnit::Unit1 => 1.0 / 3.0,
            WindowUnit::Unit2 => 2.0 / 3.0,
            WindowUnit::Unit3 => 1.0,
            WindowUnit::Frame => 1.0,
        }
    }

    /// Cycle to the next size (1u → 2u → 3u → 1u).
    /// Frame is toggled separately.
    pub fn cycle_next(&self) -> Self {
        match self {
            WindowUnit::Unit1 => WindowUnit::Unit2,
            WindowUnit::Unit2 => WindowUnit::Unit3,
            WindowUnit::Unit3 => WindowUnit::Unit1,
            WindowUnit::Frame => WindowUnit::Frame, // Frame doesn't cycle
        }
    }

    /// Whether this is an immersion/fullscreen mode.
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, WindowUnit::Frame)
    }

    /// Whether this window should have decorations.
    pub fn has_decorations(&self) -> bool {
        !matches!(self, WindowUnit::Frame)
    }
}

/// Workspace type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceType {
    /// Regular workspace (1-6) - allows 1u/2u/3u/Frame windows
    Regular,
    /// Immersion workspace (7-8) - forces Frame mode, no decorations
    Immersion,
}

impl WorkspaceType {
    /// Get the workspace type for a given workspace index (0-indexed).
    pub fn from_index(idx: usize) -> Self {
        if idx >= IMMERSION_START {
            WorkspaceType::Immersion
        } else {
            WorkspaceType::Regular
        }
    }

    /// Whether this workspace forces fullscreen mode.
    pub fn forces_fullscreen(&self) -> bool {
        matches!(self, WorkspaceType::Immersion)
    }

    /// Whether windows on this workspace should have decorations.
    pub fn allows_decorations(&self) -> bool {
        matches!(self, WorkspaceType::Regular)
    }
}

/// Check if a workspace index is valid (0-7).
pub fn is_valid_workspace_index(idx: usize) -> bool {
    idx < WORKSPACE_COUNT
}

/// Check if a workspace index is an immersion workspace.
pub fn is_immersion_workspace(idx: usize) -> bool {
    idx >= IMMERSION_START && idx < WORKSPACE_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_unit_cycle() {
        assert_eq!(WindowUnit::Unit1.cycle_next(), WindowUnit::Unit2);
        assert_eq!(WindowUnit::Unit2.cycle_next(), WindowUnit::Unit3);
        assert_eq!(WindowUnit::Unit3.cycle_next(), WindowUnit::Unit1);
    }

    #[test]
    fn test_workspace_type() {
        assert_eq!(WorkspaceType::from_index(0), WorkspaceType::Regular);
        assert_eq!(WorkspaceType::from_index(5), WorkspaceType::Regular);
        assert_eq!(WorkspaceType::from_index(6), WorkspaceType::Immersion);
        assert_eq!(WorkspaceType::from_index(7), WorkspaceType::Immersion);
    }

    #[test]
    fn test_immersion_check() {
        assert!(!is_immersion_workspace(0));
        assert!(!is_immersion_workspace(5));
        assert!(is_immersion_workspace(6));
        assert!(is_immersion_workspace(7));
        assert!(!is_immersion_workspace(8)); // Out of bounds
    }
}
