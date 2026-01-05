//! Wallpaper rendering for osvwm compositor.
//!
//! Loads images and renders them as background textures.

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::gles::GlesTexture;
use smithay::backend::renderer::utils::{CommitCounter, OpaqueRegions};
use smithay::backend::renderer::{ContextId, Frame as _, ImportMem, Renderer, Texture};
use smithay::utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Size, Transform};
use std::path::Path;
use tracing::{error, info, warn};

/// Wallpaper display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallpaperMode {
    /// Scale to fill, crop if needed (default)
    #[default]
    Fill,
    /// Scale to fit, may have letterboxing
    Fit,
    /// Stretch to exact size
    Stretch,
    /// Center without scaling
    Center,
    /// Tile the image
    Tile,
}

/// Wallpaper buffer holding the image texture
#[derive(Debug, Clone)]
pub struct WallpaperBuffer<T: Texture> {
    id: Id,
    commit_counter: CommitCounter,
    renderer_context_id: ContextId<T>,
    texture: T,
    scale: Scale<f64>,
    transform: Transform,
    /// Original image dimensions
    image_size: Size<i32, Buffer>,
}

/// Wallpaper render element
#[derive(Debug, Clone)]
pub struct WallpaperRenderElement<T: Texture> {
    buffer: WallpaperBuffer<T>,
    /// Output size to render to
    output_size: Size<i32, Physical>,
    /// Display mode
    mode: WallpaperMode,
    alpha: f32,
    kind: Kind,
}

impl<T: Texture> WallpaperBuffer<T> {
    /// Create wallpaper buffer from raw RGBA data
    pub fn from_rgba<R: Renderer<TextureId = T> + ImportMem>(
        renderer: &mut R,
        data: &[u8],
        width: u32,
        height: u32,
        scale: impl Into<Scale<f64>>,
    ) -> Result<Self, R::Error> {
        let size = Size::from((width as i32, height as i32));
        let texture = renderer.import_memory(data, Fourcc::Abgr8888, size, false)?;

        Ok(WallpaperBuffer {
            id: Id::new(),
            commit_counter: CommitCounter::default(),
            renderer_context_id: renderer.context_id(),
            texture,
            scale: scale.into(),
            transform: Transform::Normal,
            image_size: size,
        })
    }

    /// Load wallpaper from image file
    #[cfg(feature = "image")]
    pub fn from_file<R: Renderer<TextureId = T> + ImportMem>(
        renderer: &mut R,
        path: impl AsRef<Path>,
        scale: impl Into<Scale<f64>>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref();
        info!("Loading wallpaper from: {}", path.display());

        let img = image::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to load image: {}", e))?
            .into_rgba8();

        let (width, height) = img.dimensions();
        let data = img.into_raw();

        Self::from_rgba(renderer, &data, width, height, scale)
            .map_err(|_| anyhow::anyhow!("Failed to import wallpaper texture"))
    }

    pub fn texture(&self) -> &T {
        &self.texture
    }

    pub fn image_size(&self) -> Size<i32, Buffer> {
        self.image_size
    }

    pub fn logical_size(&self) -> Size<f64, Logical> {
        self.image_size
            .to_f64()
            .to_logical(self.scale, self.transform)
    }
}

impl<T: Texture> WallpaperRenderElement<T> {
    /// Create wallpaper render element
    pub fn new(
        buffer: WallpaperBuffer<T>,
        output_size: Size<i32, Physical>,
        mode: WallpaperMode,
        alpha: f32,
    ) -> Self {
        WallpaperRenderElement {
            buffer,
            output_size,
            mode,
            alpha,
            kind: Kind::Unspecified,
        }
    }

    /// Calculate source and destination rectangles for the given mode
    fn calculate_geometry(&self) -> (Rectangle<f64, Buffer>, Rectangle<i32, Physical>) {
        let img_size = self.buffer.image_size.to_f64();
        let out_size = self.output_size.to_f64();

        let (src, dst) = match self.mode {
            WallpaperMode::Fill => {
                // Scale to fill, crop if needed
                let img_aspect = img_size.w / img_size.h;
                let out_aspect = out_size.w / out_size.h;

                if img_aspect > out_aspect {
                    // Image is wider, crop sides
                    let crop_w = img_size.h * out_aspect;
                    let x = (img_size.w - crop_w) / 2.0;
                    let src = Rectangle::new(Point::from((x, 0.0)), Size::from((crop_w, img_size.h)));
                    let dst = Rectangle::from_size(self.output_size);
                    (src, dst)
                } else {
                    // Image is taller, crop top/bottom
                    let crop_h = img_size.w / out_aspect;
                    let y = (img_size.h - crop_h) / 2.0;
                    let src = Rectangle::new(Point::from((0.0, y)), Size::from((img_size.w, crop_h)));
                    let dst = Rectangle::from_size(self.output_size);
                    (src, dst)
                }
            }
            WallpaperMode::Fit => {
                // Scale to fit, letterboxing
                let img_aspect = img_size.w / img_size.h;
                let out_aspect = out_size.w / out_size.h;

                let src = Rectangle::from_size(img_size);

                let (dst_w, dst_h) = if img_aspect > out_aspect {
                    // Image is wider, fit width
                    let h = (out_size.w / img_aspect) as i32;
                    (self.output_size.w, h)
                } else {
                    // Image is taller, fit height
                    let w = (out_size.h * img_aspect) as i32;
                    (w, self.output_size.h)
                };

                let x = (self.output_size.w - dst_w) / 2;
                let y = (self.output_size.h - dst_h) / 2;
                let dst = Rectangle::new(Point::from((x, y)), Size::from((dst_w, dst_h)));
                (src, dst)
            }
            WallpaperMode::Stretch => {
                // Stretch to exact size
                let src = Rectangle::from_size(img_size);
                let dst = Rectangle::from_size(self.output_size);
                (src, dst)
            }
            WallpaperMode::Center => {
                // Center without scaling
                let src = Rectangle::from_size(img_size);
                let x = (self.output_size.w - self.buffer.image_size.w) / 2;
                let y = (self.output_size.h - self.buffer.image_size.h) / 2;
                let dst = Rectangle::new(
                    Point::from((x, y)),
                    Size::from((self.buffer.image_size.w, self.buffer.image_size.h)),
                );
                (src, dst)
            }
            WallpaperMode::Tile => {
                // For now, just center - tiling is more complex
                let src = Rectangle::from_size(img_size);
                let dst = Rectangle::from_size(self.output_size);
                (src, dst)
            }
        };

        (src, dst)
    }
}

impl<T: Texture> Element for WallpaperRenderElement<T> {
    fn id(&self) -> &Id {
        &self.buffer.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.buffer.commit_counter
    }

    fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
        Rectangle::from_size(self.output_size)
    }

    fn transform(&self) -> Transform {
        self.buffer.transform
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        let (src, _) = self.calculate_geometry();
        src
    }

    fn opaque_regions(&self, _scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        // Wallpaper is fully opaque
        vec![Rectangle::from_size(self.output_size)].into()
    }

    fn alpha(&self) -> f32 {
        self.alpha
    }

    fn kind(&self) -> Kind {
        self.kind
    }
}

impl<R, T> RenderElement<R> for WallpaperRenderElement<T>
where
    R: Renderer<TextureId = T>,
    T: Texture,
{
    fn draw(
        &self,
        frame: &mut R::Frame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        dest: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), R::Error> {
        if frame.context_id() != self.buffer.renderer_context_id {
            warn!("trying to render wallpaper texture from different renderer");
            return Ok(());
        }

        let (src, _) = self.calculate_geometry();

        frame.render_texture_from_to(
            &self.buffer.texture,
            src,
            dest,
            damage,
            opaque_regions,
            self.buffer.transform,
            self.alpha,
        )
    }

    fn underlying_storage(&self, _renderer: &mut R) -> Option<UnderlyingStorage<'_>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallpaper_mode_default() {
        assert_eq!(WallpaperMode::default(), WallpaperMode::Fill);
    }
}
