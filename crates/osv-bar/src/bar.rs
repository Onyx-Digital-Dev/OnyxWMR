//! Bar state and rendering logic.
//!
//! Implements the Phase 2 bar according to directives:
//! - Workspace capsules with window icons
//! - Auto-sizing capsules based on window count
//! - "Onyx OSV" centered, time/date right
//! - Thin, floating, 3D gradient + shadow
//! - Inter font, clean/professional

use crate::colors::*;
use crate::text;
use chrono::Local;
use tiny_skia::{
    FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point, Rect, SpreadMode,
    Transform,
};

// ============================================================================
// BAR DIMENSIONS - Thin, floating design
// ============================================================================

/// Bar height - thin as per directive
pub const BAR_HEIGHT: u32 = 24;

/// Vertical margin from top of screen (floating)
pub const BAR_MARGIN_TOP: u32 = 6;

/// Horizontal margin from screen edges
pub const BAR_MARGIN_HORIZONTAL: u32 = 8;

/// Corner radius for floating bar
pub const BAR_CORNER_RADIUS: f32 = 6.0;

/// Shadow offset
pub const SHADOW_OFFSET_Y: f32 = 2.0;

/// Shadow blur radius (simulated)
pub const SHADOW_BLUR: f32 = 4.0;

// ============================================================================
// WORKSPACE CAPSULES
// ============================================================================

/// Capsule height
pub const CAPSULE_HEIGHT: u32 = 16;

/// Minimum capsule width (empty workspace)
pub const CAPSULE_MIN_WIDTH: u32 = 20;

/// Width added per window in capsule
pub const CAPSULE_WIDTH_PER_WINDOW: u32 = 12;

/// Gap between capsules
pub const CAPSULE_GAP: u32 = 4;

/// Capsule corner radius
pub const CAPSULE_RADIUS: f32 = 4.0;

/// Capsule padding from bar edge
pub const CAPSULE_PADDING: u32 = 4;

// ============================================================================
// TYPOGRAPHY
// ============================================================================

/// Font size for branding text
pub const BRAND_FONT_SIZE: u32 = 11;

/// Font size for time
pub const TIME_FONT_SIZE: u32 = 11;

/// Number of fixed workspaces (matching osvwm)
pub const WORKSPACE_COUNT: usize = 8;

/// First immersion workspace (0-indexed)
pub const IMMERSION_START: usize = 6;

/// Workspace state
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspaceState {
    /// Whether this workspace is currently focused
    pub active: bool,
    /// Number of windows on this workspace
    pub window_count: u32,
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
            if self.active_workspace < WORKSPACE_COUNT {
                self.workspaces[self.active_workspace].active = false;
            }
            self.workspaces[idx].active = true;
            self.active_workspace = idx;
        }
    }

    /// Set window count for a workspace
    pub fn set_window_count(&mut self, idx: usize, count: u32) {
        if idx < WORKSPACE_COUNT {
            self.workspaces[idx].window_count = count;
        }
    }

    /// Set whether a workspace has urgent windows
    pub fn set_urgent(&mut self, idx: usize, urgent: bool) {
        if idx < WORKSPACE_COUNT {
            self.workspaces[idx].urgent = urgent;
        }
    }
}

/// Calculate capsule width based on window count
fn capsule_width(window_count: u32) -> u32 {
    CAPSULE_MIN_WIDTH + window_count.min(5) * CAPSULE_WIDTH_PER_WINDOW
}

/// Render the bar to a pixmap
pub fn render_bar(width: u32, height: u32, state: &BarState) -> Pixmap {
    let mut pixmap = Pixmap::new(width, height).expect("Failed to create pixmap");

    // Clear to transparent
    pixmap.fill(tiny_skia::Color::TRANSPARENT);

    // Calculate bar rectangle (floating with margins)
    let bar_x = BAR_MARGIN_HORIZONTAL as f32;
    let bar_y = BAR_MARGIN_TOP as f32;
    let bar_width = width as f32 - 2.0 * BAR_MARGIN_HORIZONTAL as f32;
    let bar_height = BAR_HEIGHT as f32;

    // Draw shadow first (behind bar)
    draw_shadow(&mut pixmap, bar_x, bar_y, bar_width, bar_height);

    // Draw bar background with 3D gradient
    draw_bar_background(&mut pixmap, bar_x, bar_y, bar_width, bar_height);

    // Draw workspace capsules on the left
    draw_workspace_capsules(&mut pixmap, bar_x, bar_y, bar_height, state);

    // Draw "Onyx OSV" branding in center
    draw_branding(&mut pixmap, bar_x, bar_y, bar_width, bar_height);

    // Draw time/date on the right
    draw_time(&mut pixmap, bar_x + bar_width, bar_y, bar_height);

    pixmap
}

/// Draw floating shadow
fn draw_shadow(pixmap: &mut Pixmap, x: f32, y: f32, width: f32, height: f32) {
    let shadow_y = y + SHADOW_OFFSET_Y;

    if let Some(path) = rounded_rect_path(x, shadow_y, width, height, BAR_CORNER_RADIUS) {
        let mut paint = Paint::default();
        paint.set_color(SHADOW_COLOR.to_skia());
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

/// Draw bar background with 3D gradient effect
fn draw_bar_background(pixmap: &mut Pixmap, x: f32, y: f32, width: f32, height: f32) {
    if let Some(path) = rounded_rect_path(x, y, width, height, BAR_CORNER_RADIUS) {
        // Create vertical gradient for 3D effect
        let gradient = LinearGradient::new(
            Point::from_xy(0.0, y),
            Point::from_xy(0.0, y + height),
            vec![
                GradientStop::new(0.0, BAR_GRADIENT_TOP.to_skia()),
                GradientStop::new(0.5, BAR_GRADIENT_MID.to_skia()),
                GradientStop::new(1.0, BAR_GRADIENT_BOTTOM.to_skia()),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        );

        if let Some(shader) = gradient {
            let mut paint = Paint::default();
            paint.shader = shader;
            paint.anti_alias = true;
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }

        // Draw subtle top highlight
        if let Some(highlight_path) = rounded_rect_path(x, y, width, 1.0, BAR_CORNER_RADIUS) {
            let mut highlight_paint = Paint::default();
            highlight_paint.set_color(BAR_HIGHLIGHT.to_skia());
            highlight_paint.anti_alias = true;
            pixmap.fill_path(
                &highlight_path,
                &highlight_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }
}

/// Draw workspace capsules
fn draw_workspace_capsules(
    pixmap: &mut Pixmap,
    bar_x: f32,
    bar_y: f32,
    bar_height: f32,
    state: &BarState,
) {
    let capsule_y = bar_y + (bar_height - CAPSULE_HEIGHT as f32) / 2.0;
    let mut x = bar_x + CAPSULE_PADDING as f32;

    for (i, ws) in state.workspaces.iter().enumerate() {
        let width = capsule_width(ws.window_count) as f32;
        let is_immersion = i >= IMMERSION_START;

        // Determine capsule color
        let color = if ws.urgent {
            WORKSPACE_URGENT
        } else if ws.active {
            if is_immersion {
                WORKSPACE_IMMERSION
            } else {
                WORKSPACE_ACTIVE
            }
        } else if ws.window_count > 0 {
            WORKSPACE_OCCUPIED
        } else if is_immersion {
            CAPSULE_IMMERSION_EMPTY
        } else {
            CAPSULE_EMPTY
        };

        // Draw capsule
        if let Some(path) =
            rounded_rect_path(x, capsule_y, width, CAPSULE_HEIGHT as f32, CAPSULE_RADIUS)
        {
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

        // Draw window dots inside capsule
        if ws.window_count > 0 {
            draw_window_dots(
                pixmap,
                x,
                capsule_y,
                width,
                CAPSULE_HEIGHT as f32,
                ws.window_count,
                ws.active,
            );
        }

        x += width + CAPSULE_GAP as f32;
    }
}

/// Draw window indicator dots inside a capsule
fn draw_window_dots(
    pixmap: &mut Pixmap,
    capsule_x: f32,
    capsule_y: f32,
    capsule_width: f32,
    capsule_height: f32,
    count: u32,
    active: bool,
) {
    let dot_radius = 2.0_f32;
    let dot_gap = 4.0_f32;
    let dots_to_show = count.min(5) as usize;
    let total_dots_width = dots_to_show as f32 * (dot_radius * 2.0 + dot_gap) - dot_gap;
    let start_x = capsule_x + (capsule_width - total_dots_width) / 2.0;
    let center_y = capsule_y + capsule_height / 2.0;

    let dot_color = if active { ONYX_BLACK } else { ONYX_BRIGHT };

    for i in 0..dots_to_show {
        let cx = start_x + i as f32 * (dot_radius * 2.0 + dot_gap) + dot_radius;
        draw_circle(
            pixmap,
            cx - dot_radius,
            center_y - dot_radius,
            dot_radius * 2.0,
            dot_color,
        );
    }
}

/// Draw "Onyx OSV" branding in center
fn draw_branding(pixmap: &mut Pixmap, bar_x: f32, bar_y: f32, bar_width: f32, bar_height: f32) {
    let center_x = bar_x + bar_width / 2.0;
    // Baseline positioned in center of bar
    let baseline_y = bar_y + bar_height / 2.0 + BRAND_FONT_SIZE as f32 / 3.0;

    if let Some(renderer) = text::get_renderer() {
        renderer.render_text_centered(
            pixmap,
            "Onyx OSV",
            center_x,
            baseline_y,
            BRAND_FONT_SIZE as f32,
            BRAND_TEXT,
        );
    } else {
        // Fallback: draw a subtle centered indicator
        let indicator_width = 60.0_f32;
        let indicator_height = 2.0_f32;
        let x = bar_x + (bar_width - indicator_width) / 2.0;
        let y = bar_y + (bar_height - indicator_height) / 2.0;

        if let Some(rect) = Rect::from_xywh(x, y, indicator_width, indicator_height) {
            let mut paint = Paint::default();
            paint.set_color(BRAND_TEXT.to_skia());
            paint.anti_alias = true;
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }
}

/// Draw time on the right
fn draw_time(pixmap: &mut Pixmap, right_edge: f32, bar_y: f32, bar_height: f32) {
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let date_str = now.format("%a %d").to_string();

    // Baseline positioned in center of bar
    let baseline_y = bar_y + bar_height / 2.0 + TIME_FONT_SIZE as f32 / 3.0;
    let padding = CAPSULE_PADDING as f32 + 4.0;

    if let Some(renderer) = text::get_renderer() {
        // Draw time
        renderer.render_text_right(
            pixmap,
            &time_str,
            right_edge - padding,
            baseline_y,
            TIME_FONT_SIZE as f32,
            TIME_TEXT,
        );

        // Draw date to the left of time
        let time_width = renderer.measure_text(&time_str, TIME_FONT_SIZE as f32);
        renderer.render_text_right(
            pixmap,
            &date_str,
            right_edge - padding - time_width - 8.0,
            baseline_y,
            TIME_FONT_SIZE as f32,
            DATE_TEXT,
        );
    } else {
        // Fallback: subtle time indicator
        let indicator_width = 40.0_f32;
        let indicator_height = 2.0_f32;
        let x = right_edge - indicator_width - padding;
        let y = bar_y + (bar_height - indicator_height) / 2.0;

        if let Some(rect) = Rect::from_xywh(x, y, indicator_width, indicator_height) {
            let mut paint = Paint::default();
            paint.set_color(TIME_TEXT.to_skia());
            paint.anti_alias = true;
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }
}

/// Create a rounded rectangle path
fn rounded_rect_path(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> Option<tiny_skia::Path> {
    let r = radius.min(width / 2.0).min(height / 2.0);
    let mut pb = PathBuilder::new();

    // Start at top-left after curve
    pb.move_to(x + r, y);

    // Top edge and top-right corner
    pb.line_to(x + width - r, y);
    pb.quad_to(x + width, y, x + width, y + r);

    // Right edge and bottom-right corner
    pb.line_to(x + width, y + height - r);
    pb.quad_to(x + width, y + height, x + width - r, y + height);

    // Bottom edge and bottom-left corner
    pb.line_to(x + r, y + height);
    pb.quad_to(x, y + height, x, y + height - r);

    // Left edge and top-left corner
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);

    pb.close();
    pb.finish()
}

/// Draw a filled circle
fn draw_circle(pixmap: &mut Pixmap, x: f32, y: f32, size: f32, color: Color) {
    let center_x = x + size / 2.0;
    let center_y = y + size / 2.0;
    let radius = size / 2.0;

    let mut pb = PathBuilder::new();
    let k = 0.5522847498_f32;
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
    fn test_capsule_width() {
        assert_eq!(capsule_width(0), CAPSULE_MIN_WIDTH);
        assert_eq!(
            capsule_width(1),
            CAPSULE_MIN_WIDTH + CAPSULE_WIDTH_PER_WINDOW
        );
        assert_eq!(
            capsule_width(5),
            CAPSULE_MIN_WIDTH + 5 * CAPSULE_WIDTH_PER_WINDOW
        );
        // Capped at 5 windows
        assert_eq!(
            capsule_width(10),
            CAPSULE_MIN_WIDTH + 5 * CAPSULE_WIDTH_PER_WINDOW
        );
    }

    #[test]
    fn test_render_bar() {
        let state = BarState::default();
        let total_height = BAR_HEIGHT + BAR_MARGIN_TOP + SHADOW_BLUR as u32;
        let pixmap = render_bar(1920, total_height, &state);
        assert_eq!(pixmap.width(), 1920);
    }
}
