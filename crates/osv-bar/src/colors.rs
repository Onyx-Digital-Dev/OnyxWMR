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

/// Success/active color - green
pub const ACCENT_SUCCESS: Color = Color::rgb(102, 204, 102);

/// Warning color - amber
pub const ACCENT_WARNING: Color = Color::rgb(255, 183, 77);

/// Error/urgent color - coral red
pub const ACCENT_ERROR: Color = Color::rgb(255, 99, 99);

/// Immersion workspace indicator - purple
pub const ACCENT_IMMERSION: Color = Color::rgb(178, 102, 255);

// ============================================================================
// BAR FLOATING DESIGN - 3D Gradient + Shadow
// ============================================================================

/// Bar gradient top (lighter for 3D effect)
pub const BAR_GRADIENT_TOP: Color = Color::new(48, 48, 56, 245);

/// Bar gradient middle
pub const BAR_GRADIENT_MID: Color = Color::new(32, 32, 40, 245);

/// Bar gradient bottom (darker for 3D effect)
pub const BAR_GRADIENT_BOTTOM: Color = Color::new(22, 22, 28, 245);

/// Bar top highlight (subtle shine)
pub const BAR_HIGHLIGHT: Color = Color::new(255, 255, 255, 15);

/// Floating shadow color
pub const SHADOW_COLOR: Color = Color::new(0, 0, 0, 80);

// ============================================================================
// WORKSPACE CAPSULES
// ============================================================================

/// Capsule - empty regular workspace
pub const CAPSULE_EMPTY: Color = Color::new(60, 60, 70, 180);

/// Capsule - empty immersion workspace (subtle purple tint)
pub const CAPSULE_IMMERSION_EMPTY: Color = Color::new(70, 50, 90, 180);

/// Workspace capsule - inactive with windows
pub const WORKSPACE_INACTIVE: Color = ONYX_MUTED;

/// Workspace capsule - active
pub const WORKSPACE_ACTIVE: Color = ACCENT_PRIMARY;

/// Workspace capsule - has windows (not active)
pub const WORKSPACE_OCCUPIED: Color = ONYX_TEXT;

/// Workspace capsule - immersion active
pub const WORKSPACE_IMMERSION: Color = ACCENT_IMMERSION;

/// Workspace capsule - urgent
pub const WORKSPACE_URGENT: Color = ACCENT_ERROR;

// ============================================================================
// TYPOGRAPHY COLORS
// ============================================================================

/// "Onyx OSV" branding text
pub const BRAND_TEXT: Color = Color::new(180, 180, 190, 200);

/// Time display text
pub const TIME_TEXT: Color = ONYX_TEXT;

/// Date display text (slightly muted)
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
        assert_eq!(pre.alpha(), 128);
    }

    #[test]
    fn test_bar_gradient_has_transparency() {
        assert!(BAR_GRADIENT_TOP.a < 255);
        assert!(BAR_GRADIENT_MID.a < 255);
        assert!(BAR_GRADIENT_BOTTOM.a < 255);
    }
}
