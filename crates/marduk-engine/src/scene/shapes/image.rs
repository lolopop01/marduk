use crate::coords::{CornerRadii, Rect};
use crate::image::ImageId;
use crate::scene::{DrawCmd, DrawList, ZIndex};

/// Image draw payload.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageCmd {
    pub rect: Rect,
    pub image_id: ImageId,
    /// UV coordinates of the top-left corner (0..1).
    pub uv_min: [f32; 2],
    /// UV coordinates of the bottom-right corner (0..1).
    pub uv_max: [f32; 2],
    /// Straight (non-premultiplied) RGBA tint. `[1,1,1,1]` = no tint.
    pub tint: [f32; 4],
    pub corner_radii: CornerRadii,
}

impl DrawList {
    /// Records an image draw command.
    ///
    /// - `uv_min` / `uv_max` control which portion of the texture is sampled
    ///   (use `[0,0]` / `[1,1]` for the full image).
    /// - `tint_straight` is a straight-alpha RGBA multiplier applied per-pixel
    ///   in the shader; pass `[1,1,1,1]` for no tint.
    #[allow(clippy::too_many_arguments)]
    pub fn push_image(
        &mut self,
        z: ZIndex,
        rect: Rect,
        image_id: ImageId,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        tint_straight: [f32; 4],
        corner_radii: CornerRadii,
    ) {
        let scale = self.current_transform().1;
        let rect  = self.tx_rect(rect);
        let corner_radii = CornerRadii {
            top_left:     corner_radii.top_left     * scale,
            top_right:    corner_radii.top_right    * scale,
            bottom_right: corner_radii.bottom_right * scale,
            bottom_left:  corner_radii.bottom_left  * scale,
        };
        self.push(
            z,
            DrawCmd::Image(ImageCmd {
                rect,
                image_id,
                uv_min,
                uv_max,
                tint: tint_straight,
                corner_radii,
            }),
        );
    }
}
