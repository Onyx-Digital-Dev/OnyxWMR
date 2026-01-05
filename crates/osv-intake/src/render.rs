//! Rendering logic for the intake launcher.
//!
//! Renders the search box and results list to a pixmap.

use crate::colors::*;
use crate::text;
use crate::IntakeState;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

// ============================================================================
// LAYOUT CONSTANTS
// ============================================================================

/// Launcher width (centered on screen)
pub const LAUNCHER_WIDTH: u32 = 600;

/// Launcher max height
pub const LAUNCHER_MAX_HEIGHT: u32 = 400;

/// Corner radius
pub const CORNER_RADIUS: f32 = 12.0;

/// Padding inside launcher
pub const PADDING: u32 = 16;

/// Search box height
pub const SEARCH_HEIGHT: u32 = 48;

/// Result item height
pub const RESULT_HEIGHT: u32 = 40;

/// Max visible results
pub const MAX_RESULTS: usize = 8;

/// Font size for search
pub const SEARCH_FONT_SIZE: f32 = 18.0;

/// Font size for results
pub const RESULT_FONT_SIZE: f32 = 14.0;

/// Font size for result description
pub const DESC_FONT_SIZE: f32 = 11.0;

/// Render the launcher UI
pub fn render_launcher(state: &IntakeState) -> Pixmap {
    let visible_results = state.results.len().min(MAX_RESULTS);
    let results_height = visible_results as u32 * RESULT_HEIGHT;
    let total_height = PADDING * 2
        + SEARCH_HEIGHT
        + if visible_results > 0 {
            PADDING + results_height
        } else {
            0
        };

    let mut pixmap = Pixmap::new(LAUNCHER_WIDTH, total_height).expect("Failed to create pixmap");

    // Clear to transparent
    pixmap.fill(tiny_skia::Color::TRANSPARENT);

    // Draw background
    draw_background(&mut pixmap, LAUNCHER_WIDTH as f32, total_height as f32);

    // Draw search box
    draw_search_box(&mut pixmap, &state.query);

    // Draw results
    if visible_results > 0 {
        draw_results(&mut pixmap, state, visible_results);
    }

    pixmap
}

/// Draw rounded background
fn draw_background(pixmap: &mut Pixmap, width: f32, height: f32) {
    if let Some(path) = rounded_rect_path(0.0, 0.0, width, height, CORNER_RADIUS) {
        let mut paint = Paint::default();
        paint.set_color(INTAKE_BG.to_skia());
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

/// Draw search box
fn draw_search_box(pixmap: &mut Pixmap, query: &str) {
    let x = PADDING as f32;
    let y = PADDING as f32;
    let width = LAUNCHER_WIDTH as f32 - PADDING as f32 * 2.0;
    let height = SEARCH_HEIGHT as f32;

    // Search box background
    if let Some(path) = rounded_rect_path(x, y, width, height, 8.0) {
        let mut paint = Paint::default();
        paint.set_color(SEARCH_BG.to_skia());
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    // Text
    if let Some(renderer) = text::get_renderer() {
        let text_x = x + 12.0;
        let text_y = y + height / 2.0 + SEARCH_FONT_SIZE / 3.0;

        if query.is_empty() {
            // Placeholder
            renderer.render_text(
                pixmap,
                "Search applications...",
                text_x,
                text_y,
                SEARCH_FONT_SIZE,
                PLACEHOLDER_TEXT,
            );
        } else {
            // Query text
            renderer.render_text(pixmap, query, text_x, text_y, SEARCH_FONT_SIZE, SEARCH_TEXT);
        }
    }
}

/// Draw result items
fn draw_results(pixmap: &mut Pixmap, state: &IntakeState, visible_count: usize) {
    let start_y = PADDING as f32 * 2.0 + SEARCH_HEIGHT as f32;

    for (i, &app_idx) in state.results.iter().take(visible_count).enumerate() {
        let y = start_y + i as u32 as f32 * RESULT_HEIGHT as f32;
        let is_selected = i == state.selected;

        draw_result_item(pixmap, &state.apps, app_idx, y, is_selected);
    }
}

/// Draw single result item
fn draw_result_item(
    pixmap: &mut Pixmap,
    apps: &crate::apps::AppList,
    app_idx: usize,
    y: f32,
    selected: bool,
) {
    let x = PADDING as f32;
    let width = LAUNCHER_WIDTH as f32 - PADDING as f32 * 2.0;
    let height = RESULT_HEIGHT as f32;

    // Selection background
    if selected {
        if let Some(path) = rounded_rect_path(x, y, width, height, 6.0) {
            let mut paint = Paint::default();
            paint.set_color(RESULT_BG_SELECTED.to_skia());
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

    // App name
    if let Some(app) = apps.get(app_idx) {
        if let Some(renderer) = text::get_renderer() {
            let text_x = x + 12.0;
            let text_y = y + height / 2.0 + RESULT_FONT_SIZE / 3.0;

            let text_color = if selected {
                RESULT_TEXT_SELECTED
            } else {
                RESULT_TEXT
            };

            renderer.render_text(
                pixmap,
                &app.name,
                text_x,
                text_y,
                RESULT_FONT_SIZE,
                text_color,
            );

            // Generic name or exec
            if let Some(ref generic) = app.generic_name {
                let name_width = renderer.measure_text(&app.name, RESULT_FONT_SIZE);
                renderer.render_text(
                    pixmap,
                    generic,
                    text_x + name_width + 12.0,
                    text_y,
                    DESC_FONT_SIZE,
                    RESULT_DESC,
                );
            }
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

    pb.move_to(x + r, y);
    pb.line_to(x + width - r, y);
    pb.quad_to(x + width, y, x + width, y + r);
    pb.line_to(x + width, y + height - r);
    pb.quad_to(x + width, y + height, x + width - r, y + height);
    pb.line_to(x + r, y + height);
    pb.quad_to(x, y + height, x, y + height - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();

    pb.finish()
}
