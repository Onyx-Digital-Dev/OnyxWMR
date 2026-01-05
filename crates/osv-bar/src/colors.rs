//! OSV Color Palette
//!
//! The official color palette for Onyx OSV components.
//! These colors are used consistently across osv-bar, osvwm, and other OSV tools.

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

    /// Convert to premultiplied RGBA for tiny-skia
    pub fn to_premultiplied(&self) -> tiny_skia::PremultipliedColorU8 {
        tiny_skia::PremultipliedColorU8::from_rgba(
            (self.r as u16 * self.a as u16 / 255) as u8,
            (self.g as u16 * self.a as u16 / 255) as u8,
            (self.b as u16 * self.a as u16 / 255) as u8,
            self.a,
        )
        .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT)
    }
}

// ============================================================================
// OSV COLOR PALETTE
// ============================================================================

/// Deep black background - main compositor/bar background
pub const ONYX_BLACK: Color = Color::rgb(18, 18, 22);

/// Slightly lighter black for contrast elements
pub const ONYX_DARK: Color = Color::rgb(28, 28, 34);

/// Surface color for panels and cards
pub const ONYX_SURFACE: Color = Color::rgb(38, 38, 46);

/// Border/separator color
pub const ONYX_BORDER: Color = Color::rgb(58, 58, 68);

/// Muted text color
pub const ONYX_MUTED: Color = Color::rgb(128, 128, 140);

/// Primary text color
pub const ONYX_TEXT: Color = Color::rgb(220, 220, 228);

/// Bright text/highlight
pub const ONYX_BRIGHT: Color = Color::rgb(248, 248, 252);

// Accent colors

/// Primary accent - vibrant blue
pub const ACCENT_PRIMARY: Color = Color::rgb(88, 166, 255);

/// Secondary accent - teal/cyan
pub const ACCENT_SECONDARY: Color = Color::rgb(78, 201, 176);

/// Success/active color - green
pub const ACCENT_SUCCESS: Color = Color::rgb(102, 204, 102);

/// Warning color - amber
pub const ACCENT_WARNING: Color = Color::rgb(255, 183, 77);

/// Error/urgent color - coral red
pub const ACCENT_ERROR: Color = Color::rgb(255, 99, 99);

/// Immersion workspace indicator - purple
pub const ACCENT_IMMERSION: Color = Color::rgb(178, 102, 255);

// ============================================================================
// BAR-SPECIFIC COLORS
// ============================================================================

/// Bar background color (semi-transparent black)
pub const BAR_BG: Color = Color::new(18, 18, 22, 230);

/// Workspace indicator - inactive
pub const WORKSPACE_INACTIVE: Color = ONYX_MUTED;

/// Workspace indicator - active
pub const WORKSPACE_ACTIVE: Color = ACCENT_PRIMARY;

/// Workspace indicator - has windows
pub const WORKSPACE_OCCUPIED: Color = ONYX_TEXT;

/// Workspace indicator - immersion (7-8)
pub const WORKSPACE_IMMERSION: Color = ACCENT_IMMERSION;

/// Workspace indicator - urgent
pub const WORKSPACE_URGENT: Color = ACCENT_ERROR;

/// Clock text color
pub const CLOCK_TEXT: Color = ONYX_TEXT;

/// Date text color (slightly muted)
pub const DATE_TEXT: Color = ONYX_MUTED;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_skia() {
        let c = Color::rgb(255, 128, 64);
        let skia = c.to_skia();
        assert_eq!(skia.red(), 255);
        assert_eq!(skia.green(), 128);
        assert_eq!(skia.blue(), 64);
        assert_eq!(skia.alpha(), 255);
    }

    #[test]
    fn test_premultiplied() {
        let c = Color::new(200, 100, 50, 128);
        let pre = c.to_premultiplied();
        // Premultiplied: r * a / 255
        assert_eq!(pre.alpha(), 128);
    }
}
