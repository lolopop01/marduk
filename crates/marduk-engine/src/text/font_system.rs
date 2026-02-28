use crate::coords::Vec2;

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
    pub fn load_font(&mut self, bytes: &[u8]) -> Result<FontId, &'static str> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())?;
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
        use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};

        let Some(font) = self.get(id) else {
            return Vec2::new(0.0, size * 1.2);
        };

        let mut layout: Layout<()> = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings { max_width, ..LayoutSettings::default() });
        layout.append(&[font], &TextStyle::new(text, size, 0));

        let glyphs = layout.glyphs();
        if glyphs.is_empty() {
            return Vec2::new(0.0, size * 1.2);
        }

        let w = glyphs.iter().map(|g| g.x + g.width as f32).fold(0.0f32, f32::max);
        let h = glyphs.iter().map(|g| g.y + g.height as f32).fold(size, f32::max);
        Vec2::new(w, h)
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}
