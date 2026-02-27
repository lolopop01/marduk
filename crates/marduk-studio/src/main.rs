use anyhow::Result;

use marduk_engine::coords::{CornerRadii, Rect, Vec2};
use marduk_engine::core::{App, AppControl, FrameCtx};
use marduk_engine::device::GpuInit;
use marduk_engine::logging::{init_logging, LoggingConfig};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::paint::gradient::{ColorStop, LinearGradient, SpreadMode};
use marduk_engine::render::shapes::circle::CircleRenderer;
use marduk_engine::render::shapes::rect::RectRenderer;
use marduk_engine::render::shapes::rounded_rect::RoundedRectRenderer;
use marduk_engine::render::shapes::text::TextRenderer;
use marduk_engine::scene::{Border, DrawList, ZIndex};
use marduk_engine::text::{FontId, FontSystem};
use marduk_engine::window::{Runtime, RuntimeConfig};

// ─────────────────────────────────────────────────────────────────────────────
// "Windows 11 dark mode Explorer" inspired mock UI.
// Pure draw-list (rects / rounded rects / gradients / text).
//
// Notes:
// - This is NOT pixel-perfect; it’s a convincer UI to show engine capability.
// - No icons are used; we fake icons with simple shapes and text glyphs.
// - If you want real icons, you’ll want a sprite/image renderer later.
// ─────────────────────────────────────────────────────────────────────────────

struct StudioApp {
    draw_list:   DrawList,
    font_system: FontSystem,
    font:        Option<FontId>,

    // Renderers — must persist across frames so GPU pipelines, buffers, and
    // the glyph atlas are not destroyed and recreated every frame.
    rect_renderer:          RectRenderer,
    rounded_rect_renderer:  RoundedRectRenderer,
    circle_renderer:        CircleRenderer,
    text_renderer:          TextRenderer,

    // Simple UI state for "selection" and fake scrolling.
    selected: usize,
    scroll_y: f32,
}

impl StudioApp {
    fn new() -> Self {
        let mut font_system = FontSystem::new();

        let font = [
            "/usr/share/fonts/TTF/SegoeUI.ttf",
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
            draw_list: DrawList::new(),
            font_system,
            font,
            rect_renderer:         RectRenderer::new(),
            rounded_rect_renderer: RoundedRectRenderer::new(),
            circle_renderer:       CircleRenderer::new(),
            text_renderer:         TextRenderer::new(),
            selected: 2,
            scroll_y: 0.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Colors (approximate Windows 11 dark)
// ─────────────────────────────────────────────────────────────────────────────
#[allow(non_snake_case)]
mod win11 {
    use super::*;

    pub fn bg() -> Color {
        Color::from_straight(0.07, 0.07, 0.09, 1.0) // app background
    }
    pub fn surface() -> Color {
        Color::from_straight(0.10, 0.10, 0.12, 1.0) // main surfaces
    }
    pub fn surface2() -> Color {
        Color::from_straight(0.12, 0.12, 0.15, 1.0) // slightly lighter
    }
    pub fn surface3() -> Color {
        Color::from_straight(0.14, 0.14, 0.18, 1.0) // hover-ish
    }
    pub fn stroke() -> Color {
        Color::from_straight(1.0, 1.0, 1.0, 0.08)
    }
    pub fn stroke_strong() -> Color {
        Color::from_straight(1.0, 1.0, 1.0, 0.14)
    }
    pub fn text() -> Color {
        Color::from_straight(1.0, 1.0, 1.0, 0.90)
    }
    pub fn text_dim() -> Color {
        Color::from_straight(1.0, 1.0, 1.0, 0.60)
    }
    pub fn text_faint() -> Color {
        Color::from_straight(1.0, 1.0, 1.0, 0.38)
    }
    pub fn accent() -> Color {
        // Windows-ish blue accent
        Color::from_straight(0.22, 0.56, 1.0, 1.0)
    }
    pub fn accent_soft() -> Color {
        Color::from_straight(0.22, 0.56, 1.0, 0.16)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tiny helpers
// ─────────────────────────────────────────────────────────────────────────────
fn rr(r: f32) -> CornerRadii {
    CornerRadii::all(r)
}

fn push_round_panel(
    dl: &mut DrawList,
    z: i32,
    rect: Rect,
    radius: f32,
    fill: Color,
    border: Color,
) {
    dl.push_rounded_rect(
        ZIndex::new(z),
        rect,
        rr(radius),
        Paint::Solid(fill),
        Some(Border::new(1.0, border)),
    );
}

fn push_round_panel2(
    dl: &mut DrawList,
    z: i32,
    rect: Rect,
    radii: CornerRadii,
    fill: Paint,
    border: Color,
    border_w: f32,
) {
    dl.push_rounded_rect(
        ZIndex::new(z),
        rect,
        radii,
        fill,
        Some(Border::new(border_w, border)),
    );
}

fn push_text(
    dl: &mut DrawList,
    z: i32,
    font: FontId,
    pos: Vec2,
    text: &str,
    size: f32,
    color: Color,
) {
    dl.push_text(ZIndex::new(z), text, font, size, color, pos, None);
}

fn ellipsize(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i + 1 >= max_chars {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

// Fake "icon" as a rounded square with a glyph inside.
fn icon_glyph(
    dl: &mut DrawList,
    font: Option<FontId>,
    rect: Rect,
    bg: Color,
    border: Color,
    glyph: &str,
) {
    push_round_panel(dl, 20, rect, 6.0, bg, border);
    if let Some(f) = font {
        // Center-ish
        let x = rect.origin.x + rect.size.x * 0.5 - 4.0;
        let y = rect.origin.y + rect.size.y * 0.5 - 8.0;
        push_text(dl, 21, f, Vec2::new(x, y), glyph, 14.0, win11::text());
    }
}

// Row hover/selection background
fn list_row_bg(dl: &mut DrawList, rect: Rect, selected: bool) {
    if selected {
        // subtle accent gradient
        dl.push_rounded_rect(
            ZIndex::new(9),
            rect,
            rr(8.0),
            Paint::LinearGradient(LinearGradient::new(
                Vec2::new(rect.origin.x, rect.origin.y),
                Vec2::new(rect.origin.x + rect.size.x, rect.origin.y),
                vec![
                    ColorStop::new(0.0, win11::accent_soft()),
                    ColorStop::new(1.0, Color::from_straight(1.0, 1.0, 1.0, 0.04)),
                ],
                SpreadMode::Pad,
            )),
            Some(Border::new(1.0, Color::from_straight(0.22, 0.56, 1.0, 0.30))),
        );
    } else {
        dl.push_rounded_rect(
            ZIndex::new(9),
            rect,
            rr(8.0),
            Paint::Solid(Color::from_straight(1.0, 1.0, 1.0, 0.02)),
            None,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fake file data
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
struct FileRow {
    name: &'static str,
    kind: &'static str,
    date: &'static str,
    size: &'static str,
}

const FILES: &[FileRow] = &[
    FileRow { name: "Design",                    kind: "Folder",        date: "2026-02-24", size: "" },
    FileRow { name: "Screenshots",               kind: "Folder",        date: "2026-02-21", size: "" },
    FileRow { name: "marduk_explorer_mock.rs",   kind: "Rust Source",   date: "2026-02-25", size: "18 KB" },
    FileRow { name: "release_notes.md",          kind: "Markdown",      date: "2026-02-20", size: "6 KB" },
    FileRow { name: "pitch_deck_v3.pptx",        kind: "PowerPoint",    date: "2026-02-18", size: "4.2 MB" },
    FileRow { name: "window_capture_01.png",     kind: "PNG Image",     date: "2026-02-17", size: "1.8 MB" },
    FileRow { name: "window_capture_02.png",     kind: "PNG Image",     date: "2026-02-17", size: "1.9 MB" },
    FileRow { name: "assets.zip",                kind: "ZIP Archive",   date: "2026-02-10", size: "92 MB" },
    FileRow { name: "README.txt",                kind: "Text Document", date: "2026-01-28", size: "2 KB" },
    FileRow { name: "meeting_recording.mp4",     kind: "MP4 Video",     date: "2026-01-15", size: "312 MB" },
    FileRow { name: "budget_2026.xlsx",          kind: "Excel",         date: "2026-01-08", size: "54 KB" },
    FileRow { name: "installer_win64.msi",       kind: "Installer",     date: "2025-12-19", size: "128 MB" },
];

// ─────────────────────────────────────────────────────────────────────────────
// App
// ─────────────────────────────────────────────────────────────────────────────
impl App for StudioApp {
    fn on_frame(&mut self, ctx: &mut FrameCtx<'_, '_>) -> AppControl {
        self.draw_list.clear();

        let (w, h) = ctx.window.logical_size();
        let pad = 16.0_f32;

        // Background
        self.draw_list.push_solid_rect(
            ZIndex::new(0),
            Rect::new(0.0, 0.0, w, h),
            win11::bg(),
        );

        // Main window "shell"
        let shell = Rect::new(pad, pad, w - pad * 2.0, h - pad * 2.0);
        push_round_panel(
            &mut self.draw_list,
            1,
            shell,
            14.0,
            win11::surface(),
            win11::stroke(),
        );

        // Top bars sizes
        let title_h = 44.0_f32;
        let cmd_h   = 54.0_f32;
        let crumb_h = 46.0_f32;

        // Sidebar
        let sidebar_w = 246.0_f32;

        let titlebar = Rect::new(shell.origin.x, shell.origin.y, shell.size.x, title_h);
        let command  = Rect::new(shell.origin.x, shell.origin.y + title_h, shell.size.x, cmd_h);
        let crumbbar = Rect::new(shell.origin.x, shell.origin.y + title_h + cmd_h, shell.size.x, crumb_h);

        // Content region
        let content_y = shell.origin.y + title_h + cmd_h + crumb_h;
        let content_h = shell.size.y - (title_h + cmd_h + crumb_h);
        let sidebar = Rect::new(shell.origin.x, content_y, sidebar_w, content_h);
        let main    = Rect::new(shell.origin.x + sidebar_w, content_y, shell.size.x - sidebar_w, content_h);

        // Subtle separators
        self.draw_list.push_solid_rect(ZIndex::new(2), Rect::new(command.origin.x, command.origin.y, command.size.x, 1.0), win11::stroke());
        self.draw_list.push_solid_rect(ZIndex::new(2), Rect::new(crumbbar.origin.x, crumbbar.origin.y, crumbbar.size.x, 1.0), win11::stroke());
        self.draw_list.push_solid_rect(ZIndex::new(2), Rect::new(sidebar.origin.x + sidebar.size.x, sidebar.origin.y, 1.0, sidebar.size.y), win11::stroke());

        // Slightly different surfaces
        self.draw_list.push_solid_rect(ZIndex::new(1), titlebar, win11::surface());
        self.draw_list.push_solid_rect(ZIndex::new(1), command,  win11::surface());
        self.draw_list.push_solid_rect(ZIndex::new(1), crumbbar, win11::surface2());
        self.draw_list.push_solid_rect(ZIndex::new(1), sidebar,  win11::surface());
        self.draw_list.push_solid_rect(ZIndex::new(1), main,     win11::surface());

        // Header "glass" highlight (tiny gradient overlay)
        self.draw_list.push_rect(
            ZIndex::new(3),
            Rect::new(shell.origin.x, shell.origin.y, shell.size.x, title_h + cmd_h),
            Paint::LinearGradient(LinearGradient::new(
                Vec2::new(shell.origin.x, shell.origin.y),
                Vec2::new(shell.origin.x, shell.origin.y + title_h + cmd_h),
                vec![
                    ColorStop::new(0.0, Color::from_straight(1.0, 1.0, 1.0, 0.06)),
                    ColorStop::new(1.0, Color::from_straight(1.0, 1.0, 1.0, 0.00)),
                ],
                SpreadMode::Pad,
            )),
        );

        // ── Title bar contents ────────────────────────────────────────────
        if let Some(font) = self.font {
            // App icon (fake)
            let ic = Rect::new(titlebar.origin.x + 14.0, titlebar.origin.y + 11.0, 22.0, 22.0);
            icon_glyph(
                &mut self.draw_list,
                self.font,
                ic,
                Color::from_straight(0.12, 0.15, 0.22, 1.0),
                Color::from_straight(0.22, 0.56, 1.0, 0.35),
                "▦",
            );

            push_text(
                &mut self.draw_list,
                10,
                font,
                Vec2::new(titlebar.origin.x + 46.0, titlebar.origin.y + 13.0),
                "File Explorer",
                15.5,
                win11::text(),
            );

            // Window controls (fake)
            let btn_w = 42.0;
            let y = titlebar.origin.y + 8.0;
            let x0 = titlebar.origin.x + titlebar.size.x - btn_w * 3.0 - 6.0;
            for (i, glyph) in ["—", "□", "✕"].iter().enumerate() {
                let r = Rect::new(x0 + btn_w * i as f32, y, btn_w, title_h - 16.0);
                push_round_panel(
                    &mut self.draw_list,
                    8,
                    r,
                    8.0,
                    Color::from_straight(1.0, 1.0, 1.0, 0.02),
                    Color::from_straight(1.0, 1.0, 1.0, 0.05),
                );
                push_text(
                    &mut self.draw_list,
                    9,
                    font,
                    Vec2::new(r.origin.x + 15.0, r.origin.y + 8.0),
                    glyph,
                    14.0,
                    win11::text_dim(),
                );
            }
        }

        // ── Command bar ───────────────────────────────────────────────────
        {
            let x = command.origin.x + 14.0;
            let y = command.origin.y + 10.0;

            // Left group: Back / Forward / Up
            let b = |i: usize| Rect::new(x + i as f32 * 40.0, y, 34.0, 34.0);

            icon_glyph(&mut self.draw_list, self.font, b(0), win11::surface2(), win11::stroke(), "←");
            icon_glyph(&mut self.draw_list, self.font, b(1), win11::surface2(), win11::stroke(), "→");
            icon_glyph(&mut self.draw_list, self.font, b(2), win11::surface2(), win11::stroke(), "↑");

            // Primary action buttons: New / Cut / Copy / Paste / Share / Delete
            let mut bx = x + 3.0 * 40.0 + 14.0;

            let button = |dl: &mut DrawList, font: Option<FontId>, r: Rect, label: &str| {
                push_round_panel(dl, 6, r, 10.0, win11::surface2(), win11::stroke());
                if let Some(f) = font {
                    push_text(dl, 7, f, Vec2::new(r.origin.x + 12.0, r.origin.y + 9.0), label, 13.0, win11::text_dim());
                }
            };

            for label in ["New", "Cut", "Copy", "Paste", "Share", "Delete"] {
                let r = Rect::new(bx, y, 74.0, 34.0);
                button(&mut self.draw_list, self.font, r, label);
                bx += 74.0 + 10.0;
                if bx > command.origin.x + command.size.x - 280.0 {
                    break;
                }
            }

            // Right side: "Sort" + "View" (small pills)
            let right = command.origin.x + command.size.x - 14.0;
            let pills = [
                ("Sort", 70.0),
                ("View", 70.0),
                ("⋯", 46.0),
            ];
            let mut rx = right;
            for (label, ww) in pills.iter().rev() {
                rx -= *ww;
                let r = Rect::new(rx, y, *ww - 8.0, 34.0);
                push_round_panel(&mut self.draw_list, 6, r, 10.0, win11::surface2(), win11::stroke());
                if let Some(f) = self.font {
                    push_text(
                        &mut self.draw_list,
                        7,
                        f,
                        Vec2::new(r.origin.x + 12.0, r.origin.y + 9.0),
                        label,
                        13.0,
                        win11::text_dim(),
                    );
                }
                rx -= 8.0;
            }
        }

        // ── Breadcrumb / address / search ─────────────────────────────────
        {
            let padx = 14.0;
            let y = crumbbar.origin.y + 8.0;

            let addr_w = crumbbar.size.x * 0.64;
            let addr = Rect::new(crumbbar.origin.x + padx, y, addr_w - padx * 0.5, 30.0);

            // Address bar
            push_round_panel(
                &mut self.draw_list,
                5,
                addr,
                10.0,
                win11::surface(),
                win11::stroke_strong(),
            );

            // Crumbs (fake)
            if let Some(f) = self.font {
                // little folder glyph
                push_text(&mut self.draw_list, 6, f, Vec2::new(addr.origin.x + 10.0, addr.origin.y + 7.0), "▸", 14.0, win11::text_faint());
                push_text(&mut self.draw_list, 6, f, Vec2::new(addr.origin.x + 28.0, addr.origin.y + 7.0), "This PC", 13.5, win11::text_dim());
                push_text(&mut self.draw_list, 6, f, Vec2::new(addr.origin.x + 92.0, addr.origin.y + 7.0), "›", 14.0, win11::text_faint());
                push_text(&mut self.draw_list, 6, f, Vec2::new(addr.origin.x + 110.0, addr.origin.y + 7.0), "Local Disk (C:)", 13.5, win11::text_dim());
                push_text(&mut self.draw_list, 6, f, Vec2::new(addr.origin.x + 236.0, addr.origin.y + 7.0), "›", 14.0, win11::text_faint());
                push_text(&mut self.draw_list, 6, f, Vec2::new(addr.origin.x + 254.0, addr.origin.y + 7.0), "Users", 13.5, win11::text_dim());
                push_text(&mut self.draw_list, 6, f, Vec2::new(addr.origin.x + 312.0, addr.origin.y + 7.0), "›", 14.0, win11::text_faint());
                push_text(&mut self.draw_list, 6, f, Vec2::new(addr.origin.x + 330.0, addr.origin.y + 7.0), "Zacharie", 13.5, win11::text());
            }

            // Search box
            let search = Rect::new(
                crumbbar.origin.x + addr_w + padx,
                y,
                crumbbar.origin.x + crumbbar.size.x - padx - (crumbbar.origin.x + addr_w + padx),
                30.0,
            );
            push_round_panel(
                &mut self.draw_list,
                5,
                search,
                10.0,
                win11::surface(),
                win11::stroke_strong(),
            );
            if let Some(f) = self.font {
                push_text(
                    &mut self.draw_list,
                    6,
                    f,
                    Vec2::new(search.origin.x + 10.0, search.origin.y + 7.0),
                    "🔎  Search",
                    13.5,
                    win11::text_faint(),
                );
            }
        }

        // ── Sidebar content ───────────────────────────────────────────────
        {
            let x = sidebar.origin.x + 14.0;
            let mut y = sidebar.origin.y + 16.0;

            let group_title = |dl: &mut DrawList, font: Option<FontId>, y: f32, label: &str| {
                if let Some(f) = font {
                    push_text(dl, 6, f, Vec2::new(x, y), label, 11.5, win11::text_faint());
                }
            };

            let nav_item = |dl: &mut DrawList, font: Option<FontId>, y: f32, label: &str, selected: bool, glyph: &str| {
                let r = Rect::new(sidebar.origin.x + 10.0, y, sidebar.size.x - 20.0, 34.0);
                if selected {
                    dl.push_rounded_rect(
                        ZIndex::new(6),
                        r,
                        rr(10.0),
                        Paint::Solid(Color::from_straight(1.0, 1.0, 1.0, 0.04)),
                        Some(Border::new(1.0, win11::stroke_strong())),
                    );
                    // accent bar on left
                    dl.push_solid_rect(
                        ZIndex::new(7),
                        Rect::new(r.origin.x + 2.0, r.origin.y + 6.0, 3.0, r.size.y - 12.0),
                        win11::accent(),
                    );
                }

                // icon
                let ic = Rect::new(r.origin.x + 10.0, r.origin.y + 7.0, 20.0, 20.0);
                icon_glyph(dl, font, ic, win11::surface2(), win11::stroke(), glyph);

                if let Some(f) = font {
                    push_text(
                        dl,
                        7,
                        f,
                        Vec2::new(r.origin.x + 40.0, r.origin.y + 9.0),
                        label,
                        13.0,
                        if selected { win11::text() } else { win11::text_dim() },
                    );
                }
            };

            group_title(&mut self.draw_list, self.font, y, "Quick access");
            y += 18.0;
            nav_item(&mut self.draw_list, self.font, y, "Desktop", false, "▣"); y += 38.0;
            nav_item(&mut self.draw_list, self.font, y, "Downloads", false, "↓"); y += 38.0;
            nav_item(&mut self.draw_list, self.font, y, "Documents", true,  "≣"); y += 38.0;
            nav_item(&mut self.draw_list, self.font, y, "Pictures", false,  "▦"); y += 38.0;

            y += 10.0;
            group_title(&mut self.draw_list, self.font, y, "This PC");
            y += 18.0;
            nav_item(&mut self.draw_list, self.font, y, "Local Disk (C:)", false, "◼"); y += 38.0;
            nav_item(&mut self.draw_list, self.font, y, "Data (D:)", false, "◼"); y += 38.0;
            nav_item(&mut self.draw_list, self.font, y, "Network", false, "⟂"); y += 38.0;
        }

        // ── Main area: header + list ──────────────────────────────────────
        {
            let inner = 16.0;
            let header_h = 38.0;

            // Header row (columns)
            let header = Rect::new(
                main.origin.x + inner,
                main.origin.y + inner,
                main.size.x - inner * 2.0,
                header_h,
            );

            push_round_panel(
                &mut self.draw_list,
                4,
                header,
                10.0,
                win11::surface2(),
                win11::stroke(),
            );

            // Columns (Name / Date modified / Type / Size)
            let col_name = header.origin.x + 14.0;
            let col_date = header.origin.x + header.size.x * 0.56;
            let col_type = header.origin.x + header.size.x * 0.75;
            let col_size = header.origin.x + header.size.x * 0.90;

            if let Some(f) = self.font {
                push_text(&mut self.draw_list, 5, f, Vec2::new(col_name, header.origin.y + 11.0), "Name", 12.5, win11::text_faint());
                push_text(&mut self.draw_list, 5, f, Vec2::new(col_date, header.origin.y + 11.0), "Date modified", 12.5, win11::text_faint());
                push_text(&mut self.draw_list, 5, f, Vec2::new(col_type, header.origin.y + 11.0), "Type", 12.5, win11::text_faint());
                push_text(&mut self.draw_list, 5, f, Vec2::new(col_size, header.origin.y + 11.0), "Size", 12.5, win11::text_faint());
            }

            // List area
            let list = Rect::new(
                main.origin.x + inner,
                header.origin.y + header.size.y + 10.0,
                main.size.x - inner * 2.0,
                main.size.y - inner * 2.0 - header_h - 10.0,
            );

            // Panel behind list
            push_round_panel(
                &mut self.draw_list,
                3,
                list,
                12.0,
                win11::surface2(),
                win11::stroke(),
            );

            let row_h = 40.0_f32;
            let mut y = list.origin.y + 8.0 - self.scroll_y;

            // Fake scroll bar rail
            {
                let rail = Rect::new(list.origin.x + list.size.x - 10.0, list.origin.y + 8.0, 4.0, list.size.y - 16.0);
                self.draw_list.push_rounded_rect(
                    ZIndex::new(12),
                    rail,
                    rr(2.0),
                    Paint::Solid(Color::from_straight(1.0, 1.0, 1.0, 0.06)),
                    None,
                );

                let thumb_h = (rail.size.y * 0.28).max(28.0);
                let thumb_y = rail.origin.y + (rail.size.y - thumb_h) * 0.20;
                let thumb = Rect::new(rail.origin.x, thumb_y, rail.size.x, thumb_h);
                self.draw_list.push_rounded_rect(
                    ZIndex::new(13),
                    thumb,
                    rr(2.0),
                    Paint::Solid(Color::from_straight(1.0, 1.0, 1.0, 0.16)),
                    None,
                );
            }

            // Rows
            for (i, row) in FILES.iter().enumerate() {
                // cull
                if y + row_h < list.origin.y {
                    y += row_h;
                    continue;
                }
                if y > list.origin.y + list.size.y {
                    break;
                }

                let rr = Rect::new(list.origin.x + 8.0, y, list.size.x - 16.0, row_h - 6.0);
                if i == self.selected {
                    list_row_bg(&mut self.draw_list, rr, true);
                }

                // "file icon"
                let ic = Rect::new(rr.origin.x + 10.0, rr.origin.y + 8.0, 24.0, 24.0);
                let glyph = if row.kind == "Folder" { "📁" } else { "📄" };
                icon_glyph(
                    &mut self.draw_list,
                    self.font,
                    ic,
                    Color::from_straight(1.0, 1.0, 1.0, 0.03),
                    win11::stroke(),
                    glyph,
                );

                if let Some(f) = self.font {
                    let name = ellipsize(row.name, 34);
                    push_text(
                        &mut self.draw_list,
                        11,
                        f,
                        Vec2::new(rr.origin.x + 44.0, rr.origin.y + 11.0),
                        &name,
                        13.8,
                        win11::text(),
                    );

                    push_text(
                        &mut self.draw_list,
                        11,
                        f,
                        Vec2::new(col_date, rr.origin.y + 11.0),
                        row.date,
                        13.0,
                        win11::text_dim(),
                    );

                    push_text(
                        &mut self.draw_list,
                        11,
                        f,
                        Vec2::new(col_type, rr.origin.y + 11.0),
                        row.kind,
                        13.0,
                        win11::text_dim(),
                    );

                    push_text(
                        &mut self.draw_list,
                        11,
                        f,
                        Vec2::new(col_size, rr.origin.y + 11.0),
                        row.size,
                        13.0,
                        win11::text_dim(),
                    );
                }

                // subtle divider
                self.draw_list.push_solid_rect(
                    ZIndex::new(8),
                    Rect::new(rr.origin.x + 8.0, rr.origin.y + rr.size.y + 2.0, rr.size.x - 16.0, 1.0),
                    Color::from_straight(1.0, 1.0, 1.0, 0.04),
                );

                y += row_h;
            }

            // Main footer hint / watermark
            if let Some(f) = self.font {
                push_text(
                    &mut self.draw_list,
                    6,
                    f,
                    Vec2::new(list.origin.x + 14.0, list.origin.y + list.size.y - 26.0),
                    "Mock Explorer UI — all primitives (rounded rects, gradients, text) — powered by marduk",
                    11.5,
                    win11::text_faint(),
                );
            }
        }

        // ── render ────────────────────────────────────────────────────────
        // Split borrows before the closure so the borrow checker can see that
        // the renderer fields and the draw_list/font_system are distinct.
        let dl = &mut self.draw_list;
        let fs = &self.font_system;
        let r_rect  = &mut self.rect_renderer;
        let r_rrect = &mut self.rounded_rect_renderer;
        let r_circ  = &mut self.circle_renderer;
        let r_text  = &mut self.text_renderer;

        ctx.render(win11::bg(), |rctx, target| {
            r_rect.render(rctx, target, dl);
            r_rrect.render(rctx, target, dl);
            r_circ.render(rctx, target, dl);
            r_text.render(rctx, target, dl, fs);
        })
    }
}

fn main() -> Result<()> {
    init_logging(LoggingConfig::default());
    Runtime::run(RuntimeConfig::default(), GpuInit::default(), StudioApp::new())
}