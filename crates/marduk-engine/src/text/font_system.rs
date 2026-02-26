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
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}
