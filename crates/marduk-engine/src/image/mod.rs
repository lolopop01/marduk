use std::collections::HashMap;

// ── ImageId ───────────────────────────────────────────────────────────────

/// Opaque handle to an image stored in an [`ImageStore`].
///
/// `Copy` + `Hash` + `Eq` so it can be used as a map key and passed freely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageId(u64);

// ── CpuImage ──────────────────────────────────────────────────────────────

/// CPU-side image data in premultiplied RGBA8 format.
pub struct CpuImage {
    /// Premultiplied RGBA8 pixel data, row-major, top-left origin.
    pub pixels: Vec<u8>,
    /// Physical texture width in pixels (what the GPU sees).
    pub width: u32,
    /// Physical texture height in pixels (what the GPU sees).
    pub height: u32,
    /// Widget layout width in logical pixels.
    ///
    /// For raster images this equals `width`. For SVGs rasterized at a scale
    /// factor it equals the SVG's natural width (`width / scale`).
    pub logical_width: u32,
    /// Widget layout height in logical pixels.
    pub logical_height: u32,
    /// Incremented by [`ImageStore::update`] so the renderer knows to re-upload.
    pub(crate) version: u64,
}

// ── ImageStore ────────────────────────────────────────────────────────────

/// Holds CPU-side image data. Mirrors the role of `FontSystem` for images.
///
/// The store is independent of any GPU state. The renderer (`ImageRenderer`)
/// lazily uploads each `CpuImage` to a GPU texture on first use, and
/// re-uploads whenever the `version` has advanced (e.g. after SVG re-rasterization).
pub struct ImageStore {
    next_id: u64,
    entries: HashMap<ImageId, CpuImage>,
}

impl ImageStore {
    pub fn new() -> Self {
        Self { next_id: 0, entries: HashMap::new() }
    }

    /// Insert raw premultiplied RGBA8 pixels where logical size == physical size.
    ///
    /// Use this for raster images (PNG, JPEG, …). For SVGs use [`insert_scaled`].
    pub fn insert(&mut self, pixels: Vec<u8>, width: u32, height: u32) -> ImageId {
        self.insert_scaled(pixels, width, height, width, height)
    }

    /// Insert pixels where the physical texture size differs from the logical layout size.
    ///
    /// Use this for SVGs rasterized at a scale factor:
    /// `logical_width/height` = natural SVG size (for widget layout),
    /// `width/height` = physical rasterized size (for GPU upload).
    pub fn insert_scaled(
        &mut self,
        pixels: Vec<u8>,
        width: u32,
        height: u32,
        logical_width: u32,
        logical_height: u32,
    ) -> ImageId {
        let id = ImageId(self.next_id);
        self.next_id += 1;
        self.entries.insert(
            id,
            CpuImage { pixels, width, height, logical_width, logical_height, version: 0 },
        );
        id
    }

    /// Replace the pixel data for an existing image (e.g. after SVG re-rasterization).
    ///
    /// The logical size is preserved. The version is bumped so [`ImageRenderer`]
    /// will re-upload the texture on the next frame.
    ///
    /// Does nothing if `id` is unknown.
    pub fn update(&mut self, id: ImageId, pixels: Vec<u8>, width: u32, height: u32) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.pixels = pixels;
            entry.width = width;
            entry.height = height;
            entry.version += 1;
        }
    }

    /// Return a reference to the stored image, or `None` if `id` is unknown.
    pub fn get(&self, id: ImageId) -> Option<&CpuImage> {
        self.entries.get(&id)
    }

    /// Return the **logical** `(width, height)` used for widget layout.
    ///
    /// For raster images this equals the texture size.
    /// For SVGs this is the natural SVG size regardless of rasterization scale.
    pub fn size(&self, id: ImageId) -> Option<(u32, u32)> {
        self.entries.get(&id).map(|img| (img.logical_width, img.logical_height))
    }
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}
