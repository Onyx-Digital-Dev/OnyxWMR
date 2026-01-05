//! Text rendering using fontdue.
//!
//! Shared font loading logic - searches system paths for suitable fonts.

use fontdue::{Font, FontSettings};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tiny_skia::Pixmap;

use crate::colors::Color;

/// Common font search paths
const FONT_PATHS: &[&str] = &[
    // Inter font locations
    "/usr/share/fonts/inter/Inter-Regular.ttf",
    "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
    "/usr/share/fonts/TTF/Inter-Regular.ttf",
    "/usr/share/fonts/google-inter/Inter-Regular.ttf",
    "/usr/local/share/fonts/Inter-Regular.ttf",
    "~/.local/share/fonts/Inter-Regular.ttf",
    "/usr/share/osv-bar/fonts/Inter-Regular.ttf",
    // Fallback sans-serif fonts
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
];

/// Text renderer with cached font
pub struct TextRenderer {
    font: Font,
}

impl TextRenderer {
    /// Create a new text renderer
    pub fn new() -> Option<Self> {
        for path_str in FONT_PATHS {
            let path = if path_str.starts_with('~') {
                if let Some(home) = std::env::var_os("HOME") {
                    Path::new(&home).join(&path_str[2..])
                } else {
                    continue;
                }
            } else {
                Path::new(path_str).to_path_buf()
            };

            if path.exists() {
                if let Ok(data) = fs::read(&path) {
                    if let Ok(font) = Font::from_bytes(data, FontSettings::default()) {
                        tracing::info!("Loaded font from: {}", path.display());
                        return Some(Self { font });
                    }
                }
            }
        }

        tracing::warn!("No suitable font found");
        None
    }

    /// Render text to a pixmap
    pub fn render_text(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) {
        let mut cursor_x = x;

        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, size);

            if bitmap.is_empty() {
                cursor_x += metrics.advance_width;
                continue;
            }

            let glyph_x = cursor_x as i32 + metrics.xmin;
            let glyph_y = y as i32 - metrics.ymin - metrics.height as i32;

            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let px = glyph_x + gx as i32;
                    let py = glyph_y + gy as i32;

                    if px < 0
                        || py < 0
                        || px >= pixmap.width() as i32
                        || py >= pixmap.height() as i32
                    {
                        continue;
                    }

                    let alpha = bitmap[gy * metrics.width + gx];
                    if alpha == 0 {
                        continue;
                    }

                    let src_alpha = (alpha as u16 * color.a as u16 / 255) as u8;
                    if src_alpha == 0 {
                        continue;
                    }

                    let idx = (py as u32 * pixmap.width() + px as u32) as usize * 4;
                    let pixels = pixmap.data_mut();

                    let sr = (color.r as u16 * src_alpha as u16 / 255) as u8;
                    let sg = (color.g as u16 * src_alpha as u16 / 255) as u8;
                    let sb = (color.b as u16 * src_alpha as u16 / 255) as u8;

                    let dr = pixels[idx];
                    let dg = pixels[idx + 1];
                    let db = pixels[idx + 2];
                    let da = pixels[idx + 3];

                    let inv_alpha = 255 - src_alpha;
                    let out_r = sr as u16 + (dr as u16 * inv_alpha as u16 / 255);
                    let out_g = sg as u16 + (dg as u16 * inv_alpha as u16 / 255);
                    let out_b = sb as u16 + (db as u16 * inv_alpha as u16 / 255);
                    let out_a = src_alpha as u16 + (da as u16 * inv_alpha as u16 / 255);

                    pixels[idx] = out_r.min(255) as u8;
                    pixels[idx + 1] = out_g.min(255) as u8;
                    pixels[idx + 2] = out_b.min(255) as u8;
                    pixels[idx + 3] = out_a.min(255) as u8;
                }
            }

            cursor_x += metrics.advance_width;
        }
    }

    /// Measure text width
    pub fn measure_text(&self, text: &str, size: f32) -> f32 {
        let mut width = 0.0;
        for ch in text.chars() {
            let metrics = self.font.metrics(ch, size);
            width += metrics.advance_width;
        }
        width
    }
}

/// Global text renderer
static TEXT_RENDERER: OnceLock<Option<TextRenderer>> = OnceLock::new();

/// Get the global text renderer
pub fn get_renderer() -> Option<&'static TextRenderer> {
    TEXT_RENDERER.get_or_init(|| TextRenderer::new()).as_ref()
}

/// Check if text rendering is available
pub fn is_available() -> bool {
    get_renderer().is_some()
}
