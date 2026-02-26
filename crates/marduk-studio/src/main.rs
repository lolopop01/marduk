use anyhow::Result;

use marduk_engine::coords::{CornerRadii, Rect, Vec2, Viewport};
use marduk_engine::core::{App, AppControl, FrameCtx};
use marduk_engine::device::{GpuInit, SurfaceErrorAction};
use marduk_engine::logging::{init_logging, LoggingConfig};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::paint::gradient::{ColorStop, LinearGradient, SpreadMode};
use marduk_engine::render::{RenderCtx, RenderTarget};
use marduk_engine::render::shapes::circle::CircleRenderer;
use marduk_engine::render::shapes::rect::RectRenderer;
use marduk_engine::render::shapes::rounded_rect::RoundedRectRenderer;
use marduk_engine::render::shapes::text::TextRenderer;
use marduk_engine::scene::{Border, DrawList, ZIndex};
use marduk_engine::text::{FontId, FontSystem};
use marduk_engine::window::{Runtime, RuntimeConfig};

struct StudioApp {
    rect_renderer:         RectRenderer,
    rounded_rect_renderer: RoundedRectRenderer,
    circle_renderer:       CircleRenderer,
    text_renderer:         TextRenderer,
    draw_list:             DrawList,
    font_system:           FontSystem,
    font:                  Option<FontId>,
}

impl StudioApp {
    fn new() -> Self {
        let mut font_system = FontSystem::new();

        let font = [
            "/usr/share/fonts/TTF/OpenSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        ]
        .iter()
        .find_map(|path| {
            std::fs::read(path)
                .ok()
                .and_then(|bytes| font_system.load_font(&bytes).ok())
        });

        if font.is_none() {
            log::warn!("No system font found — text will not render");
        }

        Self {
            rect_renderer:         RectRenderer::new(),
            rounded_rect_renderer: RoundedRectRenderer::new(),
            circle_renderer:       CircleRenderer::new(),
            text_renderer:         TextRenderer::new(),
            draw_list:             DrawList::new(),
            font_system,
            font,
        }
    }
}

/// Draws a small dim caption inside a card.
fn caption(draw_list: &mut DrawList, font: FontId, pos: Vec2, text: &str) {
    draw_list.push_text(
        ZIndex::new(10),
        text,
        font, 11.5,
        Color::from_straight(1.0, 1.0, 1.0, 0.38),
        pos,
        None,
    );
}

impl App for StudioApp {
    fn on_frame(&mut self, ctx: &mut FrameCtx<'_, '_>) -> AppControl {
        self.draw_list.clear();

        let phys  = ctx.window.window.inner_size();
        let scale = ctx.window.window.scale_factor();
        let logical: winit::dpi::LogicalSize<f64> = phys.to_logical(scale);
        let (w, h) = (logical.width as f32, logical.height as f32);

        // ── full-screen background ─────────────────────────────────────────
        self.draw_list.push_solid_rect(
            ZIndex::new(0),
            Rect::new(0.0, 0.0, w, h),
            Color::from_straight(0.07, 0.07, 0.11, 1.0),
        );

        // ── title bar ─────────────────────────────────────────────────────
        if let Some(font) = self.font {
            self.draw_list.push_text(
                ZIndex::new(10),
                "marduk  —  shape renderer test",
                font, 18.0,
                Color::from_straight(1.0, 1.0, 1.0, 0.75),
                Vec2::new(20.0, 15.0),
                None,
            );
        }

        // ── grid layout ───────────────────────────────────────────────────
        //  4 columns × 2 rows, each cell holds one test case.
        let pad    = 14.0_f32;
        let top    = 50.0_f32;
        let cols   = 4_usize;
        let rows   = 2_usize;
        let cell_w = (w - pad * (cols as f32 + 1.0)) / cols as f32;
        let cell_h = (h - top - pad * (rows as f32 + 1.0)) / rows as f32;

        let label_h = 26.0_f32; // reserved at bottom of each card for caption
        let inner   = 14.0_f32; // padding inside cards

        let cell_x = |col: usize| -> f32 { pad + col as f32 * (cell_w + pad) };
        let cell_y = |row: usize| -> f32 { top + pad + row as f32 * (cell_h + pad) };

        // Card backgrounds (drawn all at once before shapes).
        for row in 0..rows {
            for col in 0..cols {
                self.draw_list.push_rounded_rect(
                    ZIndex::new(1),
                    Rect::new(cell_x(col), cell_y(row), cell_w, cell_h),
                    CornerRadii::all(12.0),
                    Paint::Solid(Color::from_straight(0.13, 0.13, 0.20, 1.0)),
                    Some(Border::new(1.0, Color::from_straight(1.0, 1.0, 1.0, 0.09))),
                );
            }
        }

        // Helpers that compute the drawable area and caption position for a cell.
        let shape_area = |col: usize, row: usize| -> Rect {
            Rect::new(
                cell_x(col) + inner,
                cell_y(row) + inner,
                cell_w - inner * 2.0,
                cell_h - label_h - inner * 2.0,
            )
        };
        let caption_pos = |col: usize, row: usize| -> Vec2 {
            Vec2::new(cell_x(col) + inner, cell_y(row) + cell_h - label_h + 6.0)
        };

        // ── [0, 0]  Solid rect ────────────────────────────────────────────
        {
            let a = shape_area(0, 0);
            let ins = 18.0_f32;
            self.draw_list.push_solid_rect(
                ZIndex::new(2),
                Rect::new(a.origin.x + ins, a.origin.y + ins,
                          a.size.x - ins * 2.0, a.size.y - ins * 2.0),
                Color::from_straight(0.35, 0.55, 1.0, 1.0),
            );
            if let Some(f) = self.font {
                caption(&mut self.draw_list, f, caption_pos(0, 0), "solid rect");
            }
        }

        // ── [1, 0]  Gradient rect ─────────────────────────────────────────
        {
            let a = shape_area(1, 0);
            let ins = 18.0_f32;
            let r = Rect::new(a.origin.x + ins, a.origin.y + ins,
                              a.size.x - ins * 2.0, a.size.y - ins * 2.0);
            self.draw_list.push_rect(
                ZIndex::new(2), r,
                Paint::LinearGradient(LinearGradient::new(
                    Vec2::new(r.origin.x, r.origin.y),
                    Vec2::new(r.origin.x, r.origin.y + r.size.y),
                    vec![
                        ColorStop::new(0.0, Color::from_straight(1.0, 0.4, 0.2, 1.0)),
                        ColorStop::new(1.0, Color::from_straight(0.5, 0.1, 0.9, 1.0)),
                    ],
                    SpreadMode::Pad,
                )),
            );
            if let Some(f) = self.font {
                caption(&mut self.draw_list, f, caption_pos(1, 0), "gradient rect");
            }
        }

        // ── [2, 0]  Solid rounded rect ────────────────────────────────────
        {
            let a = shape_area(2, 0);
            let ins = 18.0_f32;
            self.draw_list.push_solid_rounded_rect(
                ZIndex::new(2),
                Rect::new(a.origin.x + ins, a.origin.y + ins,
                          a.size.x - ins * 2.0, a.size.y - ins * 2.0),
                20.0,
                Color::from_straight(0.2, 0.85, 0.55, 1.0),
            );
            if let Some(f) = self.font {
                caption(&mut self.draw_list, f, caption_pos(2, 0), "solid rounded rect  r=20");
            }
        }

        // ── [3, 0]  Rounded rect + gradient + border ──────────────────────
        {
            let a = shape_area(3, 0);
            let ins = 18.0_f32;
            let r = Rect::new(a.origin.x + ins, a.origin.y + ins,
                              a.size.x - ins * 2.0, a.size.y - ins * 2.0);
            self.draw_list.push_rounded_rect(
                ZIndex::new(2), r,
                CornerRadii::all(24.0),
                Paint::LinearGradient(LinearGradient::new(
                    Vec2::new(r.origin.x, r.origin.y),
                    Vec2::new(r.origin.x + r.size.x, r.origin.y + r.size.y),
                    vec![
                        ColorStop::new(0.0, Color::from_straight(0.95, 0.3, 0.5, 1.0)),
                        ColorStop::new(1.0, Color::from_straight(0.3, 0.2, 1.0,  1.0)),
                    ],
                    SpreadMode::Pad,
                )),
                Some(Border::new(3.0, Color::from_straight(1.0, 1.0, 1.0, 0.6))),
            );
            if let Some(f) = self.font {
                caption(&mut self.draw_list, f, caption_pos(3, 0), "rounded rect  gradient  border");
            }
        }

        // ── [0, 1]  Solid circle ──────────────────────────────────────────
        {
            let a = shape_area(0, 1);
            let r = (a.size.x.min(a.size.y) * 0.5 - 18.0).max(8.0);
            let c = Vec2::new(a.origin.x + a.size.x * 0.5, a.origin.y + a.size.y * 0.5);
            self.draw_list.push_solid_circle(
                ZIndex::new(2), c, r,
                Color::from_straight(1.0, 0.75, 0.1, 1.0),
            );
            if let Some(f) = self.font {
                caption(&mut self.draw_list, f, caption_pos(0, 1), "solid circle");
            }
        }

        // ── [1, 1]  Circle + gradient + border ───────────────────────────
        {
            let a = shape_area(1, 1);
            let r = (a.size.x.min(a.size.y) * 0.5 - 18.0).max(8.0);
            let c = Vec2::new(a.origin.x + a.size.x * 0.5, a.origin.y + a.size.y * 0.5);
            self.draw_list.push_circle(
                ZIndex::new(2), c, r,
                Paint::LinearGradient(LinearGradient::new(
                    Vec2::new(c.x, c.y - r),
                    Vec2::new(c.x, c.y + r),
                    vec![
                        ColorStop::new(0.0, Color::from_straight(0.1, 0.9, 1.0, 1.0)),
                        ColorStop::new(1.0, Color::from_straight(0.0, 0.3, 0.8, 1.0)),
                    ],
                    SpreadMode::Pad,
                )),
                Some(Border::new(3.0, Color::from_straight(1.0, 1.0, 1.0, 0.7))),
            );
            if let Some(f) = self.font {
                caption(&mut self.draw_list, f, caption_pos(1, 1), "circle  gradient  border");
            }
        }

        // ── [2, 1]  Asymmetric corner radii ──────────────────────────────
        {
            let a = shape_area(2, 1);
            let ins = 18.0_f32;
            let r = Rect::new(a.origin.x + ins, a.origin.y + ins,
                              a.size.x - ins * 2.0, a.size.y - ins * 2.0);
            // top-left=0, top-right=36, bottom-right=0, bottom-left=36
            self.draw_list.push_rounded_rect(
                ZIndex::new(2), r,
                CornerRadii::new(0.0, 36.0, 0.0, 36.0),
                Paint::Solid(Color::from_straight(0.7, 0.35, 1.0, 1.0)),
                Some(Border::new(2.0, Color::from_straight(1.0, 0.8, 1.0, 0.5))),
            );
            if let Some(f) = self.font {
                caption(&mut self.draw_list, f, caption_pos(2, 1), "asymmetric radii  (0, 36, 0, 36)");
            }
        }

        // ── [3, 1]  Text sizes ────────────────────────────────────────────
        if let Some(font) = self.font {
            let a = shape_area(3, 1);
            let x = a.origin.x + 8.0;
            let mut y = a.origin.y + 6.0;
            let bright = Color::from_straight(1.0, 1.0, 1.0, 0.88);
            let dim    = Color::from_straight(1.0, 1.0, 1.0, 0.50);

            for (text, size, color) in [
                ("28px  Large heading",    28.0_f32, bright),
                ("20px  Section title",    20.0,     bright),
                ("16px  Body text",        16.0,     dim),
                ("13px  Label / caption",  13.0,     dim),
                ("10px  Fine print",       10.0,     dim),
            ] {
                self.draw_list.push_text(
                    ZIndex::new(2), text, font, size, color,
                    Vec2::new(x, y), None,
                );
                y += size + 5.0;
            }

            caption(&mut self.draw_list, font, caption_pos(3, 1), "text  —  multiple sizes");
        }

        // ── acquire frame and render ───────────────────────────────────────
        let mut frame = match ctx.gpu.begin_frame() {
            Ok(f) => f,
            Err(err) => {
                let action = ctx.gpu.handle_surface_error(err);
                if action == SurfaceErrorAction::Fatal { return AppControl::Exit; }
                return AppControl::Continue;
            }
        };

        {
            let _rpass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.07, g: 0.07, b: 0.11, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        let viewport = Viewport::new(w, h);
        let rctx     = RenderCtx::new(ctx.gpu.device(), ctx.gpu.queue(), ctx.gpu.surface_format(), viewport);
        let mut target = RenderTarget::new(&mut frame.encoder, &frame.view);

        self.rect_renderer.render(&rctx, &mut target, &mut self.draw_list);
        self.rounded_rect_renderer.render(&rctx, &mut target, &mut self.draw_list);
        self.circle_renderer.render(&rctx, &mut target, &mut self.draw_list);
        self.text_renderer.render(&rctx, &mut target, &mut self.draw_list, &self.font_system);

        ctx.window.window.pre_present_notify();
        ctx.gpu.submit(frame);

        AppControl::Continue
    }
}

fn main() -> Result<()> {
    init_logging(LoggingConfig::default());
    Runtime::run(RuntimeConfig::default(), GpuInit::default(), StudioApp::new())
}
