//! Image decoding — raster (via `image` crate) and SVG (via `resvg`).

/// Decoded image: premultiplied RGBA8 pixels + physical + logical dimensions.
pub struct DecodedImage {
    pub pixels: Vec<u8>,
    /// Physical texture width (GPU upload size).
    pub width: u32,
    /// Physical texture height (GPU upload size).
    pub height: u32,
    /// Logical width for widget layout (= `width` for raster, = natural SVG width for SVG).
    pub logical_width: u32,
    /// Logical height for widget layout.
    pub logical_height: u32,
}

/// Decode bytes as either an SVG or a raster image.
///
/// `scale` is used only for SVG: the natural size is multiplied by `scale`
/// before rasterization (e.g. `2.0` for HiDPI icons).
/// For raster formats `scale` is ignored (always decoded at natural resolution).
pub fn decode_image(bytes: &[u8], scale: f32) -> Result<DecodedImage, String> {
    if is_svg(bytes) {
        decode_svg(bytes, scale)
    } else {
        decode_raster(bytes)
    }
}

/// Returns `true` if `bytes` appears to be an SVG document.
pub fn is_svg(bytes: &[u8]) -> bool {
    bytes.iter()
        .position(|&b| !b.is_ascii_whitespace())
        .map(|i| bytes[i] == b'<')
        .unwrap_or(false)
}

// ── raster ────────────────────────────────────────────────────────────────

fn decode_raster(bytes: &[u8]) -> Result<DecodedImage, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = premultiply(rgba.into_raw());
    // For raster images logical size == physical size.
    Ok(DecodedImage { pixels, width, height, logical_width: width, logical_height: height })
}

// ── SVG ───────────────────────────────────────────────────────────────────

pub fn decode_svg(bytes: &[u8], scale: f32) -> Result<DecodedImage, String> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default())
        .map_err(|e| e.to_string())?;
    let svg_size = tree.size();
    let logical_width  = (svg_size.width()  as u32).max(1);
    let logical_height = (svg_size.height() as u32).max(1);
    let scale = scale.max(0.01);
    let width  = ((svg_size.width()  * scale) as u32).max(1);
    let height = ((svg_size.height() * scale) as u32).max(1);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "SVG rasterized to zero-size pixmap".to_string())?;
    let transform = resvg::tiny_skia::Transform::from_scale(
        width  as f32 / svg_size.width(),
        height as f32 / svg_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    // tiny-skia outputs premultiplied RGBA.
    Ok(DecodedImage { pixels: pixmap.take(), width, height, logical_width, logical_height })
}

// ── alpha premultiplication ────────────────────────────────────────────────

fn premultiply(mut pixels: Vec<u8>) -> Vec<u8> {
    for px in pixels.chunks_exact_mut(4) {
        let a = px[3] as f32 / 255.0;
        px[0] = (px[0] as f32 * a) as u8;
        px[1] = (px[1] as f32 * a) as u8;
        px[2] = (px[2] as f32 * a) as u8;
    }
    pixels
}
