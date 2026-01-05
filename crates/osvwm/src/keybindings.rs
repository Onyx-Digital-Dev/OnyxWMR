//! Hardcoded keybindings for osvwm.
//!
//! osvwm uses a fixed set of keybindings that cannot be changed by the user.
//! This enforces muscle-memory-driven workflow.

use osvwm_config::Action as ConfigAction;
use smithay::input::keyboard::{Keysym, ModifiersState};

/// Modifier key (Super/Mod4).
pub const SUPER: ModifiersState = ModifiersState {
    ctrl: false,
    alt: false,
    shift: false,
    caps_lock: false,
    logo: true,
    num_lock: false,
    iso_level3_shift: false,
    serialized: Default::default(),
};

/// Modifier keys (Super + Shift).
pub const SUPER_SHIFT: ModifiersState = ModifiersState {
    ctrl: false,
    alt: false,
    shift: true,
    caps_lock: false,
    logo: true,
    num_lock: false,
    iso_level3_shift: false,
    serialized: Default::default(),
};

/// Actions that can be triggered by keybindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Focus the workspace above (Super+K or Super+Up)
    WorkspaceUp,
    /// Focus the workspace below (Super+J or Super+Down)
    WorkspaceDown,
    /// Focus window to the left in strip (Super+H or Super+Left)
    FocusLeft,
    /// Focus window to the right in strip (Super+L or Super+Right)
    FocusRight,
    /// Move window left in strip (Super+Shift+H)
    MoveWindowLeft,
    /// Move window right in strip (Super+Shift+L)
    MoveWindowRight,
    /// Move window to workspace below (Super+Shift+J)
    MoveWindowToWorkspaceDown,
    /// Move window to workspace above (Super+Shift+K)
    MoveWindowToWorkspaceUp,
    /// Cycle window size 1u→2u→3u→1u (Super+R)
    CycleWindowSize,
    /// Toggle Frame/fullscreen mode (Super+F)
    ToggleFrame,
    /// Toggle floating mode (Super+V)
    ToggleFloating,
    /// Open overview (Super+O)
    OpenOverview,
    /// Launch osv-intake launcher (Super+Space)
    LaunchIntake,
    /// Launch terminal - alacritty (Super+Return)
    LaunchTerminal,
    /// Close focused window (Super+Q)
    CloseWindow,
    /// Exit compositor (Super+Shift+Escape)
    ExitCompositor,
}

impl Action {
    /// Convert to osvwm_config::Action for the compositor to execute.
    pub fn to_config_action(self) -> ConfigAction {
        match self {
            Action::WorkspaceUp => ConfigAction::FocusWorkspaceUp,
            Action::WorkspaceDown => ConfigAction::FocusWorkspaceDown,
            Action::FocusLeft => ConfigAction::FocusColumnLeft,
            Action::FocusRight => ConfigAction::FocusColumnRight,
            Action::MoveWindowLeft => ConfigAction::MoveColumnLeft,
            Action::MoveWindowRight => ConfigAction::MoveColumnRight,
            Action::MoveWindowToWorkspaceDown => ConfigAction::MoveWindowToWorkspaceDown(true),
            Action::MoveWindowToWorkspaceUp => ConfigAction::MoveWindowToWorkspaceUp(true),
            Action::CycleWindowSize => ConfigAction::SwitchPresetWindowWidth,
            Action::ToggleFrame => ConfigAction::FullscreenWindow,
            Action::ToggleFloating => ConfigAction::ToggleWindowFloating,
            Action::OpenOverview => ConfigAction::OpenOverview,
            Action::LaunchIntake => ConfigAction::Spawn(vec![LAUNCHER_COMMAND.to_string()]),
            Action::LaunchTerminal => ConfigAction::Spawn(vec![TERMINAL_COMMAND.to_string()]),
            Action::CloseWindow => ConfigAction::CloseWindow,
            Action::ExitCompositor => ConfigAction::Quit(false),
        }
    }
}

/// Check if a key event matches a hardcoded binding.
/// Returns the action if matched, None otherwise.
pub fn match_keybinding(modifiers: &ModifiersState, keysym: Keysym) -> Option<Action> {
    // Super + key bindings
    if modifiers.logo && !modifiers.shift && !modifiers.ctrl && !modifiers.alt {
        match keysym {
            // Workspace navigation
            Keysym::k | Keysym::K | Keysym::Up => return Some(Action::WorkspaceUp),
            Keysym::j | Keysym::J | Keysym::Down => return Some(Action::WorkspaceDown),

            // Focus in strip
            Keysym::h | Keysym::H | Keysym::Left => return Some(Action::FocusLeft),
            Keysym::l | Keysym::L | Keysym::Right => return Some(Action::FocusRight),

            // Window sizing and modes
            Keysym::r | Keysym::R => return Some(Action::CycleWindowSize),
            Keysym::f | Keysym::F => return Some(Action::ToggleFrame),
            Keysym::v | Keysym::V => return Some(Action::ToggleFloating),

            // Overview
            Keysym::o | Keysym::O => return Some(Action::OpenOverview),

            // Launchers
            Keysym::space => return Some(Action::LaunchIntake),
            Keysym::Return => return Some(Action::LaunchTerminal),

            // Window management
            Keysym::q | Keysym::Q => return Some(Action::CloseWindow),

            _ => {}
        }
    }

    // Super + Shift + key bindings
    if modifiers.logo && modifiers.shift && !modifiers.ctrl && !modifiers.alt {
        match keysym {
            // Move window in strip
            Keysym::h | Keysym::H => return Some(Action::MoveWindowLeft),
            Keysym::l | Keysym::L => return Some(Action::MoveWindowRight),

            // Move window between workspaces
            Keysym::j | Keysym::J => return Some(Action::MoveWindowToWorkspaceDown),
            Keysym::k | Keysym::K => return Some(Action::MoveWindowToWorkspaceUp),

            // Exit compositor
            Keysym::Escape => return Some(Action::ExitCompositor),

            _ => {}
        }
    }

    None
}

/// Terminal command to spawn.
pub const TERMINAL_COMMAND: &str = "alacritty";

/// Launcher command to spawn (osv-intake).
pub const LAUNCHER_COMMAND: &str = "osv-intake";

#[cfg(test)]
mod tests {
    use super::*;

    fn super_only() -> ModifiersState {
        ModifiersState {
            logo: true,
            ..Default::default()
        }
    }

    fn super_shift() -> ModifiersState {
        ModifiersState {
            logo: true,
            shift: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_workspace_navigation() {
        assert_eq!(match_keybinding(&super_only(), Keysym::k), Some(Action::WorkspaceUp));
        assert_eq!(match_keybinding(&super_only(), Keysym::Up), Some(Action::WorkspaceUp));
        assert_eq!(match_keybinding(&super_only(), Keysym::j), Some(Action::WorkspaceDown));
        assert_eq!(match_keybinding(&super_only(), Keysym::Down), Some(Action::WorkspaceDown));
    }

    #[test]
    fn test_focus_navigation() {
        assert_eq!(match_keybinding(&super_only(), Keysym::h), Some(Action::FocusLeft));
        assert_eq!(match_keybinding(&super_only(), Keysym::Left), Some(Action::FocusLeft));
        assert_eq!(match_keybinding(&super_only(), Keysym::l), Some(Action::FocusRight));
        assert_eq!(match_keybinding(&super_only(), Keysym::Right), Some(Action::FocusRight));
    }

    #[test]
    fn test_window_movement() {
        assert_eq!(match_keybinding(&super_shift(), Keysym::h), Some(Action::MoveWindowLeft));
        assert_eq!(match_keybinding(&super_shift(), Keysym::l), Some(Action::MoveWindowRight));
        assert_eq!(match_keybinding(&super_shift(), Keysym::j), Some(Action::MoveWindowToWorkspaceDown));
        assert_eq!(match_keybinding(&super_shift(), Keysym::k), Some(Action::MoveWindowToWorkspaceUp));
    }

    #[test]
    fn test_window_sizing() {
        assert_eq!(match_keybinding(&super_only(), Keysym::r), Some(Action::CycleWindowSize));
        assert_eq!(match_keybinding(&super_only(), Keysym::f), Some(Action::ToggleFrame));
        assert_eq!(match_keybinding(&super_only(), Keysym::v), Some(Action::ToggleFloating));
    }

    #[test]
    fn test_launchers() {
        assert_eq!(match_keybinding(&super_only(), Keysym::space), Some(Action::LaunchIntake));
        assert_eq!(match_keybinding(&super_only(), Keysym::Return), Some(Action::LaunchTerminal));
    }

    #[test]
    fn test_window_close() {
        assert_eq!(match_keybinding(&super_only(), Keysym::q), Some(Action::CloseWindow));
    }

    #[test]
    fn test_exit() {
        assert_eq!(match_keybinding(&super_shift(), Keysym::Escape), Some(Action::ExitCompositor));
    }
}
