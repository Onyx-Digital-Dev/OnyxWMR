//! OSV Color Palette for intake launcher.
//!
//! Reuses the same color palette as osv-bar for consistency.

/// RGBA color representation (0-255 range)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Convert to tiny-skia Color
    pub fn to_skia(&self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

// ============================================================================
// OSV BASE COLORS
// ============================================================================

/// Deep black - darkest background
pub const ONYX_BLACK: Color = Color::rgb(18, 18, 22);

/// Slightly lighter black for contrast
pub const ONYX_DARK: Color = Color::rgb(28, 28, 34);

/// Surface color for panels
pub const ONYX_SURFACE: Color = Color::rgb(38, 38, 46);

/// Border/separator color
pub const ONYX_BORDER: Color = Color::rgb(58, 58, 68);

/// Muted text color
pub const ONYX_MUTED: Color = Color::rgb(128, 128, 140);

/// Primary text color
pub const ONYX_TEXT: Color = Color::rgb(220, 220, 228);

/// Bright text/highlight
pub const ONYX_BRIGHT: Color = Color::rgb(248, 248, 252);

// ============================================================================
// ACCENT COLORS
// ============================================================================

/// Primary accent - vibrant blue
pub const ACCENT_PRIMARY: Color = Color::rgb(88, 166, 255);

/// Secondary accent - teal/cyan
pub const ACCENT_SECONDARY: Color = Color::rgb(78, 201, 176);

// ============================================================================
// INTAKE-SPECIFIC COLORS
// ============================================================================

/// Launcher background (semi-transparent)
pub const INTAKE_BG: Color = Color::new(18, 18, 22, 240);

/// Search box background
pub const SEARCH_BG: Color = Color::new(38, 38, 46, 255);

/// Search box border
pub const SEARCH_BORDER: Color = ONYX_BORDER;

/// Search text
pub const SEARCH_TEXT: Color = ONYX_BRIGHT;

/// Placeholder text
pub const PLACEHOLDER_TEXT: Color = ONYX_MUTED;

/// Result item background (normal)
pub const RESULT_BG: Color = Color::new(0, 0, 0, 0);

/// Result item background (selected)
pub const RESULT_BG_SELECTED: Color = Color::new(88, 166, 255, 40);

/// Result item text (normal)
pub const RESULT_TEXT: Color = ONYX_TEXT;

/// Result item text (selected)
pub const RESULT_TEXT_SELECTED: Color = ONYX_BRIGHT;

/// Result item description
pub const RESULT_DESC: Color = ONYX_MUTED;
