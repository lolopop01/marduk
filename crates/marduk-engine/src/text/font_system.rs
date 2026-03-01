use std::fmt;
use crate::coords::Vec2;

/// Error returned by [`FontSystem::load_font`].
#[derive(Debug, Clone)]
pub struct FontLoadError(pub String);

impl fmt::Display for FontLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "font load error: {}", self.0)
    }
}

impl std::error::Error for FontLoadError {}

/// Opaque handle to a font loaded into a [`FontSystem`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FontId(pub(crate) usize);

/// Owns a collection of loaded fonts.
///
/// Fonts are immutable after loading. The system is owned by the application
/// and passed to [`TextRenderer::render`] each frame so new glyphs can be
/// rasterized on demand.
pub struct FontSystem {
    fonts: Vec<fontdue::Font>,
}

impl FontSystem {
    pub fn new() -> Self {
        Self { fonts: Vec::new() }
    }

    /// Parses and stores a TrueType or OpenType font from raw bytes.
    ///
    /// Returns the `FontId` that identifies the font in draw commands.
    pub fn load_font(&mut self, bytes: &[u8]) -> Result<FontId, FontLoadError> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| FontLoadError(e.to_string()))?;
        let id = FontId(self.fonts.len());
        self.fonts.push(font);
        Ok(id)
    }

    /// Returns a reference to the underlying `fontdue::Font`, if `id` is valid.
    pub(crate) fn get(&self, id: FontId) -> Option<&fontdue::Font> {
        self.fonts.get(id.0)
    }

    /// Computes the bounding box of a laid-out text string.
    ///
    /// Returns `(width, height)` in logical pixels. Used by the UI layer for
    /// layout without needing direct access to `fontdue::Font`.
    #[must_use]
    pub fn measure_text(&self, text: &str, id: FontId, size: f32, max_width: Option<f32>) -> Vec2 {
        self.measure_text_scaled(text, id, size, max_width, 1.0)
    }

    /// Like [`measure_text`] but performs the layout at `size * scale` and
    /// divides the result back to logical pixels.
    ///
    /// Pass the same `scale` value the text renderer uses (`raster_scale =
    /// os_scale * zoom`, quantised to 0.25 steps) so the returned width
    /// matches the renderer's physical-pixel glyph positions exactly.  This
    /// eliminates the cumulative per-character drift that arises when fontdue's
    /// advances at different pixel sizes are not perfectly proportional.
    #[must_use]
    pub fn measure_text_scaled(
        &self,
        text: &str,
        id: FontId,
        size: f32,
        max_width: Option<f32>,
        scale: f32,
    ) -> Vec2 {
        use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};

        let Some(font) = self.get(id) else {
            return Vec2::new(0.0, size * 1.2);
        };

        let scale = scale.max(0.01);
        let phys_size = size * scale;
        let phys_max = max_width.map(|w| w * scale);

        let mut layout: Layout<()> = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings { max_width: phys_max, ..LayoutSettings::default() });
        layout.append(&[font], &TextStyle::new(text, phys_size, 0));

        let glyphs = layout.glyphs();
        if glyphs.is_empty() {
            return Vec2::new(0.0, size * 1.2);
        }

        // Use the pen position *after* each glyph (= g.x - xmin + advance_width) rather
        // than the bitmap right edge (= g.x + g.width).  Fontdue's wrap check is:
        //   (pen_before - settings.x) + advance_width > max_width
        // so max_width must be >= the advance extent to avoid spurious wrapping in the
        // renderer when the measured width is later used as the paint max_width.
        let w = glyphs.iter().map(|g| {
            let m = font.metrics_indexed(g.key.glyph_index, phys_size);
            (g.x - m.xmin as f32 + m.advance_width).max(0.0)
        }).fold(0.0f32, f32::max) / scale;
        let h = glyphs.iter().map(|g| g.y + g.height as f32).fold(phys_size, f32::max) / scale;
        Vec2::new(w, h)
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}
