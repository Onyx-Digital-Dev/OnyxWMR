//! Bar state and rendering logic.

use crate::colors::*;
use chrono::Local;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

/// Bar configuration
pub const BAR_HEIGHT: u32 = 32;
pub const WORKSPACE_INDICATOR_SIZE: u32 = 8;
pub const WORKSPACE_INDICATOR_GAP: u32 = 12;
pub const WORKSPACE_INDICATOR_PADDING: u32 = 16;
pub const CLOCK_PADDING: u32 = 16;

/// Number of fixed workspaces (matching osvwm)
pub const WORKSPACE_COUNT: usize = 8;

/// First immersion workspace (0-indexed)
pub const IMMERSION_START: usize = 6;

/// Workspace state
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspaceState {
    /// Whether this workspace is currently focused
    pub active: bool,
    /// Whether this workspace has any windows
    pub occupied: bool,
    /// Whether this workspace has urgent windows
    pub urgent: bool,
}

/// Bar state
#[derive(Debug, Clone)]
pub struct BarState {
    /// State of each workspace (8 fixed workspaces)
    pub workspaces: [WorkspaceState; WORKSPACE_COUNT],
    /// Currently active workspace index (0-7)
    pub active_workspace: usize,
}

impl Default for BarState {
    fn default() -> Self {
        let mut workspaces = [WorkspaceState::default(); WORKSPACE_COUNT];
        workspaces[0].active = true;
        Self {
            workspaces,
            active_workspace: 0,
        }
    }
}

impl BarState {
    /// Set the active workspace
    pub fn set_active(&mut self, idx: usize) {
        if idx < WORKSPACE_COUNT {
            // Clear old active
            if self.active_workspace < WORKSPACE_COUNT {
                self.workspaces[self.active_workspace].active = false;
            }
            // Set new active
            self.workspaces[idx].active = true;
            self.active_workspace = idx;
        }
    }

    /// Set whether a workspace is occupied
    pub fn set_occupied(&mut self, idx: usize, occupied: bool) {
        if idx < WORKSPACE_COUNT {
            self.workspaces[idx].occupied = occupied;
        }
    }

    /// Set whether a workspace has urgent windows
    pub fn set_urgent(&mut self, idx: usize, urgent: bool) {
        if idx < WORKSPACE_COUNT {
            self.workspaces[idx].urgent = urgent;
        }
    }
}

/// Render the bar to a pixmap
pub fn render_bar(width: u32, height: u32, state: &BarState) -> Pixmap {
    let mut pixmap = Pixmap::new(width, height).expect("Failed to create pixmap");

    // Fill background
    let bg = BAR_BG.to_premultiplied();
    pixmap.fill(tiny_skia::Color::from_rgba8(
        bg.red(),
        bg.green(),
        bg.blue(),
        bg.alpha(),
    ));

    // Draw workspace indicators on the left
    draw_workspace_indicators(&mut pixmap, state);

    // Draw clock on the right
    draw_clock(&mut pixmap, width);

    pixmap
}

/// Draw workspace indicators
fn draw_workspace_indicators(pixmap: &mut Pixmap, state: &BarState) {
    let height = pixmap.height();
    let center_y = height as f32 / 2.0;

    for (i, ws) in state.workspaces.iter().enumerate() {
        let x = WORKSPACE_INDICATOR_PADDING as f32
            + i as f32 * (WORKSPACE_INDICATOR_SIZE as f32 + WORKSPACE_INDICATOR_GAP as f32);
        let y = center_y - WORKSPACE_INDICATOR_SIZE as f32 / 2.0;

        // Determine color based on state
        let color = if ws.urgent {
            WORKSPACE_URGENT
        } else if ws.active {
            if i >= IMMERSION_START {
                WORKSPACE_IMMERSION
            } else {
                WORKSPACE_ACTIVE
            }
        } else if ws.occupied {
            WORKSPACE_OCCUPIED
        } else if i >= IMMERSION_START {
            // Immersion workspaces get a subtle purple tint even when inactive
            Color::new(
                WORKSPACE_IMMERSION.r / 3,
                WORKSPACE_IMMERSION.g / 3,
                WORKSPACE_IMMERSION.b / 3,
                180,
            )
        } else {
            WORKSPACE_INACTIVE
        };

        // Draw indicator (circle for regular, diamond for immersion)
        if i >= IMMERSION_START {
            draw_diamond(pixmap, x, y, WORKSPACE_INDICATOR_SIZE as f32, color);
        } else {
            draw_circle(pixmap, x, y, WORKSPACE_INDICATOR_SIZE as f32, color);
        }
    }
}

/// Draw a filled circle
fn draw_circle(pixmap: &mut Pixmap, x: f32, y: f32, size: f32, color: Color) {
    let center_x = x + size / 2.0;
    let center_y = y + size / 2.0;
    let radius = size / 2.0;

    // Approximate circle with a path
    let mut pb = PathBuilder::new();

    // Use bezier curves to approximate a circle
    let k = 0.5522847498; // Magic number for circle approximation
    let r = radius;

    pb.move_to(center_x, center_y - r);
    pb.cubic_to(
        center_x + r * k,
        center_y - r,
        center_x + r,
        center_y - r * k,
        center_x + r,
        center_y,
    );
    pb.cubic_to(
        center_x + r,
        center_y + r * k,
        center_x + r * k,
        center_y + r,
        center_x,
        center_y + r,
    );
    pb.cubic_to(
        center_x - r * k,
        center_y + r,
        center_x - r,
        center_y + r * k,
        center_x - r,
        center_y,
    );
    pb.cubic_to(
        center_x - r,
        center_y - r * k,
        center_x - r * k,
        center_y - r,
        center_x,
        center_y - r,
    );
    pb.close();

    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// Draw a filled diamond (for immersion workspaces)
fn draw_diamond(pixmap: &mut Pixmap, x: f32, y: f32, size: f32, color: Color) {
    let center_x = x + size / 2.0;
    let center_y = y + size / 2.0;
    let half = size / 2.0;

    let mut pb = PathBuilder::new();
    pb.move_to(center_x, center_y - half); // Top
    pb.line_to(center_x + half, center_y); // Right
    pb.line_to(center_x, center_y + half); // Bottom
    pb.line_to(center_x - half, center_y); // Left
    pb.close();

    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(color.to_skia());
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

/// Draw the clock on the right side
/// Note: Text rendering requires fontdue, we'll draw a simple placeholder for now
fn draw_clock(pixmap: &mut Pixmap, width: u32) {
    let now = Local::now();
    let _time_str = now.format("%H:%M").to_string();
    let _date_str = now.format("%a %b %d").to_string();

    // For now, draw a simple indicator showing clock position
    // Full text rendering will be added when fontdue is integrated
    let height = pixmap.height();
    let clock_width = 80.0_f32;
    let x = width as f32 - clock_width - CLOCK_PADDING as f32;
    let y = (height as f32 - 4.0) / 2.0;

    // Draw a subtle rectangle as clock placeholder
    if let Some(rect) = Rect::from_xywh(x, y, clock_width, 4.0) {
        let mut paint = Paint::default();
        paint.set_color(ONYX_MUTED.to_skia());
        paint.anti_alias = true;
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_state_default() {
        let state = BarState::default();
        assert!(state.workspaces[0].active);
        assert_eq!(state.active_workspace, 0);
    }

    #[test]
    fn test_set_active_workspace() {
        let mut state = BarState::default();
        state.set_active(3);
        assert!(!state.workspaces[0].active);
        assert!(state.workspaces[3].active);
        assert_eq!(state.active_workspace, 3);
    }

    #[test]
    fn test_render_bar() {
        let state = BarState::default();
        let pixmap = render_bar(1920, BAR_HEIGHT, &state);
        assert_eq!(pixmap.width(), 1920);
        assert_eq!(pixmap.height(), BAR_HEIGHT);
    }
}
