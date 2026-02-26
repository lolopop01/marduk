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

        // Try common TTF paths; fall back gracefully if none is found.
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

impl App for StudioApp {
    fn on_frame(&mut self, ctx: &mut FrameCtx<'_, '_>) -> AppControl {
        self.draw_list.clear();

        let t = ctx.time.frame_index as f32 * 0.012;
        let phys  = ctx.window.window.inner_size();
        let scale = ctx.window.window.scale_factor();
        let logical: winit::dpi::LogicalSize<f64> = phys.to_logical(scale);
        let (w, h) = (logical.width as f32, logical.height as f32);
        let (cx, cy) = (w / 2.0, h / 2.0);

        // ── background card ───────────────────────────────────────────────
        let margin   = 40.0_f32;
        let card     = Rect::new(margin, margin, w - margin * 2.0, h - margin * 2.0);
        let card_grad = Paint::LinearGradient(LinearGradient::new(
            Vec2::new(card.origin.x, card.origin.y),
            Vec2::new(card.origin.x, card.origin.y + card.size.y),
            vec![
                ColorStop::new(0.0, Color::from_straight(0.10, 0.10, 0.16, 1.0)),
                ColorStop::new(1.0, Color::from_straight(0.06, 0.06, 0.10, 1.0)),
            ],
            SpreadMode::Pad,
        ));
        self.draw_list.push_rounded_rect(
            ZIndex::new(0), card, CornerRadii::all(20.0), card_grad,
            Some(Border::new(1.5, Color::from_straight(1.0, 1.0, 1.0, 0.08))),
        );

        // ── orbiting rounded-rect blobs ───────────────────────────────────
        let orbit = 110.0_f32;
        let size  = Vec2::new(180.0, 180.0);
        let half  = size * 0.5;
        let blobs: [(f32, Color); 3] = [
            (0.0,                               Color::from_straight(1.0,  0.15, 0.15, 0.65)),
            (std::f32::consts::TAU / 3.0,       Color::from_straight(0.15, 1.0,  0.25, 0.65)),
            (2.0 * std::f32::consts::TAU / 3.0, Color::from_straight(0.2,  0.5,  1.0,  0.65)),
        ];
        for (phase, color) in blobs {
            let angle  = t + phase;
            let origin = Vec2::new(cx + orbit * angle.cos() - half.x,
                                   cy + orbit * angle.sin() - half.y);
            self.draw_list.push_solid_rounded_rect(
                ZIndex::new(1),
                Rect::from_origin_size(origin, size),
                28.0, color,
            );
        }

        // ── gradient circles ──────────────────────────────────────────────
        let circle_y = cy + 200.0;
        let radii    = [18.0_f32, 24.0, 30.0, 24.0, 18.0];
        let spacing  = 80.0_f32;
        let total_w  = (radii.len() as f32 - 1.0) * spacing;
        let start_x  = cx - total_w / 2.0;
        for (i, &r) in radii.iter().enumerate() {
            let center = Vec2::new(start_x + i as f32 * spacing, circle_y);
            let hue_t  = (t * 0.5 + i as f32 * std::f32::consts::TAU / radii.len() as f32)
                .sin() * 0.5 + 0.5;
            let paint = Paint::LinearGradient(LinearGradient::new(
                Vec2::new(center.x, center.y - r),
                Vec2::new(center.x, center.y + r),
                vec![
                    ColorStop::new(0.0, Color::from_straight(0.3 + 0.7 * hue_t, 0.4, 1.0 - 0.7 * hue_t, 1.0)),
                    ColorStop::new(1.0, Color::from_straight(0.1, 0.1 + 0.4 * hue_t, 0.6, 1.0)),
                ],
                SpreadMode::Pad,
            ));
            self.draw_list.push_circle(
                ZIndex::new(2), center, r, paint,
                Some(Border::new(2.0, Color::from_straight(1.0, 1.0, 1.0, 0.5))),
            );
        }

        // ── indicator dots ────────────────────────────────────────────────
        for i in 0..5 {
            let x     = cx + (i as f32 - 2.0) * 20.0;
            let y     = h - margin - 28.0;
            let alpha = if i == ((t * 0.4) as usize % 5) { 1.0 } else { 0.25 };
            self.draw_list.push_solid_circle(
                ZIndex::new(3),
                Vec2::new(x, y), 5.0,
                Color::from_straight(1.0, 1.0, 1.0, alpha),
            );
        }

        // ── text labels ───────────────────────────────────────────────────
        if let Some(font) = self.font {
            let white   = Color::from_straight(1.0, 1.0, 1.0, 0.9);
            let subtext = Color::from_straight(1.0, 1.0, 1.0, 0.5);

            self.draw_list.push_text(
                ZIndex::new(4),
                "marduk",
                font, 40.0, white,
                Vec2::new(margin + 32.0, margin + 32.0),
                None,
            );
            self.draw_list.push_text(
                ZIndex::new(4),
                "GPU UI renderer  ·  wgpu + fontdue",
                font, 16.0, subtext,
                Vec2::new(margin + 32.0, margin + 80.0),
                None,
            );

            // FPS counter at bottom-right.
            let fps_str = format!("frame {}", ctx.time.frame_index);
            self.draw_list.push_text(
                ZIndex::new(4),
                fps_str,
                font, 13.0,
                Color::from_straight(1.0, 1.0, 1.0, 0.3),
                Vec2::new(w - margin - 160.0, h - margin - 24.0),
                None,
            );
        }

        // ── acquire frame ─────────────────────────────────────────────────
        let mut frame = match ctx.gpu.begin_frame() {
            Ok(f) => f,
            Err(err) => {
                let action = ctx.gpu.handle_surface_error(err);
                if action == SurfaceErrorAction::Fatal { return AppControl::Exit; }
                return AppControl::Continue;
            }
        };

        // Clear.
        {
            let _rpass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("marduk-studio clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.03, g: 0.03, b: 0.05, a: 1.0 }),
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
