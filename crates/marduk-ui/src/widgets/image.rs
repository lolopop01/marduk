use marduk_engine::coords::{CornerRadii, Rect, Vec2};
use marduk_engine::image::ImageId;
use marduk_engine::paint::Color;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

// ── ImageFit ──────────────────────────────────────────────────────────────

/// Controls how an image is scaled and cropped to fill its widget rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFit {
    /// Stretch the image to exactly fill the widget rect, ignoring aspect ratio.
    Fill,
    /// Scale the image to fit within the rect while preserving aspect ratio.
    /// Adds letterbox / pillarbox space as needed.
    #[default]
    Contain,
    /// Scale the image to cover the entire rect while preserving aspect ratio.
    /// Crops the image center if needed.
    Cover,
    /// Display the image at its natural pixel size without scaling.
    None,
}

// ── Image widget ──────────────────────────────────────────────────────────

/// A widget that renders an image stored in the UI scene's [`ImageStore`].
///
/// # Example
/// ```rust,ignore
/// Image::new(logo_id)
///     .fit(ImageFit::Contain)
///     .corner_radius(8.0)
/// ```
pub struct Image {
    id: ImageId,
    fit: ImageFit,
    /// Premultiplied tint color. Default: opaque white (no tint).
    tint: Color,
    radii: CornerRadii,
}

impl Image {
    pub fn new(id: ImageId) -> Self {
        Self {
            id,
            fit: ImageFit::Contain,
            tint: Color::from_straight(1.0, 1.0, 1.0, 1.0),
            radii: CornerRadii::all(0.0),
        }
    }

    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }

    /// Set a premultiplied tint color.
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = color;
        self
    }

    /// Set a uniform corner radius.
    pub fn corner_radius(mut self, r: f32) -> Self {
        self.radii = CornerRadii::all(r);
        self
    }

    /// Set per-corner radii.
    pub fn corner_radii(mut self, radii: CornerRadii) -> Self {
        self.radii = radii;
        self
    }
}

impl Widget for Image {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let (nw, nh) = ctx.images.size(self.id).unwrap_or((1, 1));
        let natural = Vec2::new(nw as f32, nh as f32);

        match self.fit {
            ImageFit::None => {
                // Clamp natural size to constraints.
                constraints.constrain(natural)
            }
            ImageFit::Fill | ImageFit::Cover => {
                // Takes up as much space as given.
                constraints.constrain(constraints.max)
            }
            ImageFit::Contain => {
                // Fit natural size inside max constraint, preserving aspect ratio.
                let max = constraints.max;
                if max.x.is_infinite() && max.y.is_infinite() {
                    return constraints.constrain(natural);
                }
                let scale_x = if max.x.is_infinite() { f32::INFINITY } else { max.x / natural.x.max(1.0) };
                let scale_y = if max.y.is_infinite() { f32::INFINITY } else { max.y / natural.y.max(1.0) };
                let scale = scale_x.min(scale_y).min(1.0); // don't upscale beyond natural
                constraints.constrain(Vec2::new(natural.x * scale, natural.y * scale))
            }
        }
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let (nw, nh) = painter.image_store.size(self.id).unwrap_or((1, 1));
        let nw = nw as f32;
        let nh = nh as f32;

        let (dest_rect, uv_min, uv_max) = compute_fit(self.fit, rect, nw, nh);

        // Convert premultiplied tint back to straight for the shader.
        let t = self.tint;
        let tint_straight = if t.a > 0.0 {
            [t.r / t.a, t.g / t.a, t.b / t.a, t.a]
        } else {
            [0.0, 0.0, 0.0, 0.0]
        };

        painter.draw_image(dest_rect, self.id, uv_min, uv_max, tint_straight, self.radii);
    }

    fn on_event(&mut self, _event: &UiEvent, _rect: Rect, _ctx: &LayoutCtx) -> EventResult {
        EventResult::Ignored
    }
}

// ── fit computation ───────────────────────────────────────────────────────

fn compute_fit(
    fit: ImageFit,
    rect: Rect,
    nw: f32,
    nh: f32,
) -> (Rect, [f32; 2], [f32; 2]) {
    let rw = rect.size.x.max(1.0);
    let rh = rect.size.y.max(1.0);
    let nw = nw.max(1.0);
    let nh = nh.max(1.0);

    match fit {
        ImageFit::Fill | ImageFit::None => {
            (rect, [0.0, 0.0], [1.0, 1.0])
        }
        ImageFit::Contain => {
            let scale = f32::min(rw / nw, rh / nh);
            let dw = nw * scale;
            let dh = nh * scale;
            let ox = rect.origin.x + (rw - dw) * 0.5;
            let oy = rect.origin.y + (rh - dh) * 0.5;
            (Rect::new(ox, oy, dw, dh), [0.0, 0.0], [1.0, 1.0])
        }
        ImageFit::Cover => {
            let scale = f32::max(rw / nw, rh / nh);
            let sw = rw / (nw * scale);
            let sh = rh / (nh * scale);
            let u0 = (1.0 - sw) * 0.5;
            let v0 = (1.0 - sh) * 0.5;
            (rect, [u0, v0], [u0 + sw, v0 + sh])
        }
    }
}
