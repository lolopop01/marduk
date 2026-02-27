use marduk_ui::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// "Windows 11 dark mode Explorer" — rebuilt with marduk-ui widgets.
//
// All rendering goes through `Painter` — no direct engine imports needed.
// ─────────────────────────────────────────────────────────────────────────────

// ── Colors ────────────────────────────────────────────────────────────────

mod win11 {
    use super::*;

    pub fn bg()           -> Color { Color::from_straight(0.07, 0.07, 0.09, 1.0) }
    pub fn surface()      -> Color { Color::from_straight(0.10, 0.10, 0.12, 1.0) }
    pub fn surface2()     -> Color { Color::from_straight(0.12, 0.12, 0.15, 1.0) }
    pub fn stroke()       -> Color { Color::from_straight(1.0,  1.0,  1.0,  0.08) }
    pub fn stroke_strong()-> Color { Color::from_straight(1.0,  1.0,  1.0,  0.14) }
    pub fn text()         -> Color { Color::from_straight(1.0,  1.0,  1.0,  0.90) }
    pub fn text_dim()     -> Color { Color::from_straight(1.0,  1.0,  1.0,  0.60) }
    pub fn text_faint()   -> Color { Color::from_straight(1.0,  1.0,  1.0,  0.38) }
    pub fn accent()       -> Color { Color::from_straight(0.22, 0.56, 1.0,  1.0)  }
    pub fn accent_soft()  -> Color { Color::from_straight(0.22, 0.56, 1.0,  0.16) }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn round_panel(p: &mut Painter, rect: Rect, radius: f32, fill: Color, border: Color) {
    p.fill_rounded_rect(rect, radius, Paint::Solid(fill), Some(Border::new(1.0, border)));
}

fn push_text(p: &mut Painter, font: Option<FontId>, pos: Vec2, text: &str, size: f32, color: Color) {
    if let Some(f) = font {
        p.text(text, f, size, color, pos, None);
    }
}

fn icon_glyph(p: &mut Painter, font: Option<FontId>, rect: Rect, bg: Color, border: Color, glyph: &str) {
    round_panel(p, rect, 6.0, bg, border);
    if let Some(f) = font {
        let x = rect.origin.x + rect.size.x * 0.5 - 4.0;
        let y = rect.origin.y + rect.size.y * 0.5 - 8.0;
        p.text(glyph, f, 14.0, win11::text(), Vec2::new(x, y), None);
    }
}

fn list_row_bg(p: &mut Painter, rect: Rect, selected: bool) {
    if selected {
        p.fill_rounded_rect(
            rect,
            8.0,
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
    }
}

fn ellipsize(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars { return s.to_string(); }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i + 1 >= max_chars { break; }
        out.push(ch);
    }
    out.push('…');
    out
}

// ── Fake file data ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct FileRow { name: &'static str, kind: &'static str, date: &'static str, size: &'static str }

const FILES: &[FileRow] = &[
    FileRow { name: "Design",                  kind: "Folder",        date: "2026-02-24", size: "" },
    FileRow { name: "Screenshots",             kind: "Folder",        date: "2026-02-21", size: "" },
    FileRow { name: "marduk_explorer_mock.rs", kind: "Rust Source",   date: "2026-02-25", size: "18 KB" },
    FileRow { name: "release_notes.md",        kind: "Markdown",      date: "2026-02-20", size: "6 KB" },
    FileRow { name: "pitch_deck_v3.pptx",      kind: "PowerPoint",    date: "2026-02-18", size: "4.2 MB" },
    FileRow { name: "window_capture_01.png",   kind: "PNG Image",     date: "2026-02-17", size: "1.8 MB" },
    FileRow { name: "window_capture_02.png",   kind: "PNG Image",     date: "2026-02-17", size: "1.9 MB" },
    FileRow { name: "assets.zip",              kind: "ZIP Archive",   date: "2026-02-10", size: "92 MB" },
    FileRow { name: "README.txt",              kind: "Text Document", date: "2026-01-28", size: "2 KB" },
    FileRow { name: "meeting_recording.mp4",   kind: "MP4 Video",     date: "2026-01-15", size: "312 MB" },
    FileRow { name: "budget_2026.xlsx",        kind: "Excel",         date: "2026-01-08", size: "54 KB" },
    FileRow { name: "installer_win64.msi",     kind: "Installer",     date: "2025-12-19", size: "128 MB" },
];

// ── StudioRoot widget ─────────────────────────────────────────────────────

/// Root widget for the Windows Explorer demo.
///
/// Holds all per-frame mutable state: selection, scroll offset, font handle.
struct StudioRoot {
    font:     Option<FontId>,
    selected: usize,
    scroll_y: f32,
}

impl StudioRoot {
    fn new(font: Option<FontId>) -> Self {
        Self { font, selected: 2, scroll_y: 0.0 }
    }
}

impl Widget for StudioRoot {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        // Fill available viewport.
        constraints.max
    }

    fn paint(&self, p: &mut Painter, rect: Rect) {
        let w = rect.size.x;
        let h = rect.size.y;
        let pad = 16.0_f32;

        // Background
        p.fill_rect(rect, win11::bg());

        // Main window shell
        let shell = Rect::new(pad, pad, w - pad * 2.0, h - pad * 2.0);
        round_panel(p, shell, 14.0, win11::surface(), win11::stroke());

        // Heights
        let title_h = 44.0_f32;
        let cmd_h   = 54.0_f32;
        let crumb_h = 46.0_f32;
        let sidebar_w = 246.0_f32;

        let titlebar = Rect::new(shell.origin.x, shell.origin.y,                      shell.size.x, title_h);
        let command  = Rect::new(shell.origin.x, shell.origin.y + title_h,             shell.size.x, cmd_h);
        let crumbbar = Rect::new(shell.origin.x, shell.origin.y + title_h + cmd_h,     shell.size.x, crumb_h);

        let content_y = shell.origin.y + title_h + cmd_h + crumb_h;
        let content_h = shell.size.y   - (title_h + cmd_h + crumb_h);
        let sidebar   = Rect::new(shell.origin.x,              content_y, sidebar_w,                content_h);
        let main      = Rect::new(shell.origin.x + sidebar_w,  content_y, shell.size.x - sidebar_w, content_h);

        // Separator lines
        p.fill_rect(Rect::new(command.origin.x,  command.origin.y,  command.size.x,  1.0), win11::stroke());
        p.fill_rect(Rect::new(crumbbar.origin.x, crumbbar.origin.y, crumbbar.size.x, 1.0), win11::stroke());
        p.fill_rect(Rect::new(sidebar.origin.x + sidebar.size.x, sidebar.origin.y, 1.0, sidebar.size.y), win11::stroke());

        // Section fills
        p.fill_rect(titlebar, win11::surface());
        p.fill_rect(command,  win11::surface());
        p.fill_rect(crumbbar, win11::surface2());
        p.fill_rect(sidebar,  win11::surface());
        p.fill_rect(main,     win11::surface());

        // Header glass gradient
        p.fill_rounded_rect(
            Rect::new(shell.origin.x, shell.origin.y, shell.size.x, title_h + cmd_h),
            0.0,
            Paint::LinearGradient(LinearGradient::new(
                Vec2::new(shell.origin.x, shell.origin.y),
                Vec2::new(shell.origin.x, shell.origin.y + title_h + cmd_h),
                vec![
                    ColorStop::new(0.0, Color::from_straight(1.0, 1.0, 1.0, 0.06)),
                    ColorStop::new(1.0, Color::from_straight(1.0, 1.0, 1.0, 0.00)),
                ],
                SpreadMode::Pad,
            )),
            None,
        );

        // ── Title bar ─────────────────────────────────────────────────────
        {
            let ic = Rect::new(titlebar.origin.x + 14.0, titlebar.origin.y + 11.0, 22.0, 22.0);
            icon_glyph(p, self.font, ic,
                Color::from_straight(0.12, 0.15, 0.22, 1.0),
                Color::from_straight(0.22, 0.56, 1.0, 0.35),
                "▦");

            push_text(p, self.font,
                Vec2::new(titlebar.origin.x + 46.0, titlebar.origin.y + 13.0),
                "File Explorer", 15.5, win11::text());

            // Window control buttons
            let btn_w = 42.0;
            let y = titlebar.origin.y + 8.0;
            let x0 = titlebar.origin.x + titlebar.size.x - btn_w * 3.0 - 6.0;
            for (i, glyph) in ["—", "□", "✕"].iter().enumerate() {
                let r = Rect::new(x0 + btn_w * i as f32, y, btn_w, title_h - 16.0);
                round_panel(p, r, 8.0,
                    Color::from_straight(1.0, 1.0, 1.0, 0.02),
                    Color::from_straight(1.0, 1.0, 1.0, 0.05));
                push_text(p, self.font,
                    Vec2::new(r.origin.x + 15.0, r.origin.y + 8.0),
                    glyph, 14.0, win11::text_dim());
            }
        }

        // ── Command bar ───────────────────────────────────────────────────
        {
            let x = command.origin.x + 14.0;
            let y = command.origin.y + 10.0;

            // Nav buttons
            for (i, glyph) in ["←", "→", "↑"].iter().enumerate() {
                let r = Rect::new(x + i as f32 * 40.0, y, 34.0, 34.0);
                icon_glyph(p, self.font, r, win11::surface2(), win11::stroke(), glyph);
            }

            // Action buttons
            let mut bx = x + 3.0 * 40.0 + 14.0;
            for label in ["New", "Cut", "Copy", "Paste", "Share", "Delete"] {
                if bx > command.origin.x + command.size.x - 280.0 { break; }
                let r = Rect::new(bx, y, 74.0, 34.0);
                round_panel(p, r, 10.0, win11::surface2(), win11::stroke());
                push_text(p, self.font,
                    Vec2::new(r.origin.x + 12.0, r.origin.y + 9.0),
                    label, 13.0, win11::text_dim());
                bx += 84.0;
            }

            // Right pills
            let right = command.origin.x + command.size.x - 14.0;
            let mut rx = right;
            for (label, ww) in [("⋯", 46.0_f32), ("View", 70.0), ("Sort", 70.0)] {
                rx -= ww;
                let r = Rect::new(rx, y, ww - 8.0, 34.0);
                round_panel(p, r, 10.0, win11::surface2(), win11::stroke());
                push_text(p, self.font,
                    Vec2::new(r.origin.x + 12.0, r.origin.y + 9.0),
                    label, 13.0, win11::text_dim());
                rx -= 8.0;
            }
        }

        // ── Breadcrumb / search ───────────────────────────────────────────
        {
            let padx = 14.0;
            let y    = crumbbar.origin.y + 8.0;
            let addr_w = crumbbar.size.x * 0.64;
            let addr = Rect::new(crumbbar.origin.x + padx, y, addr_w - padx * 0.5, 30.0);

            round_panel(p, addr, 10.0, win11::surface(), win11::stroke_strong());
            push_text(p, self.font, Vec2::new(addr.origin.x + 10.0, addr.origin.y + 7.0), "▸", 14.0, win11::text_faint());
            push_text(p, self.font, Vec2::new(addr.origin.x + 28.0, addr.origin.y + 7.0), "This PC", 13.5, win11::text_dim());
            push_text(p, self.font, Vec2::new(addr.origin.x + 92.0, addr.origin.y + 7.0), "›", 14.0, win11::text_faint());
            push_text(p, self.font, Vec2::new(addr.origin.x + 110.0, addr.origin.y + 7.0), "Local Disk (C:)", 13.5, win11::text_dim());
            push_text(p, self.font, Vec2::new(addr.origin.x + 236.0, addr.origin.y + 7.0), "›", 14.0, win11::text_faint());
            push_text(p, self.font, Vec2::new(addr.origin.x + 254.0, addr.origin.y + 7.0), "Users", 13.5, win11::text_dim());
            push_text(p, self.font, Vec2::new(addr.origin.x + 312.0, addr.origin.y + 7.0), "›", 14.0, win11::text_faint());
            push_text(p, self.font, Vec2::new(addr.origin.x + 330.0, addr.origin.y + 7.0), "Zacharie", 13.5, win11::text());

            let search = Rect::new(
                crumbbar.origin.x + addr_w + padx, y,
                crumbbar.origin.x + crumbbar.size.x - padx - (crumbbar.origin.x + addr_w + padx),
                30.0,
            );
            round_panel(p, search, 10.0, win11::surface(), win11::stroke_strong());
            push_text(p, self.font, Vec2::new(search.origin.x + 10.0, search.origin.y + 7.0), "🔎  Search", 13.5, win11::text_faint());
        }

        // ── Sidebar ───────────────────────────────────────────────────────
        {
            let x = sidebar.origin.x + 14.0;
            let mut y = sidebar.origin.y + 16.0;

            macro_rules! group_title {
                ($label:expr) => {
                    push_text(p, self.font, Vec2::new(x, y), $label, 11.5, win11::text_faint());
                    y += 18.0;
                };
            }

            macro_rules! nav_item {
                ($label:expr, $sel:expr, $glyph:expr) => {{
                    let r = Rect::new(sidebar.origin.x + 10.0, y, sidebar.size.x - 20.0, 34.0);
                    if $sel {
                        p.fill_rounded_rect(r, 10.0,
                            Paint::Solid(Color::from_straight(1.0, 1.0, 1.0, 0.04)),
                            Some(Border::new(1.0, win11::stroke_strong())));
                        p.fill_rect(Rect::new(r.origin.x + 2.0, r.origin.y + 6.0, 3.0, r.size.y - 12.0), win11::accent());
                    }
                    let ic = Rect::new(r.origin.x + 10.0, r.origin.y + 7.0, 20.0, 20.0);
                    icon_glyph(p, self.font, ic, win11::surface2(), win11::stroke(), $glyph);
                    let color = if $sel { win11::text() } else { win11::text_dim() };
                    push_text(p, self.font, Vec2::new(r.origin.x + 40.0, r.origin.y + 9.0), $label, 13.0, color);
                    y += 38.0;
                }};
            }

            group_title!("Quick access");
            nav_item!("Desktop",   false, "▣");
            nav_item!("Downloads", false, "↓");
            nav_item!("Documents", true,  "≣");
            nav_item!("Pictures",  false, "▦");

            y += 10.0;
            group_title!("This PC");
            nav_item!("Local Disk (C:)", false, "◼");
            nav_item!("Data (D:)",       false, "◼");
            nav_item!("Network",         false, "⟂");
        }

        // ── Main content ──────────────────────────────────────────────────
        {
            let inner   = 16.0;
            let hdr_h   = 38.0;
            let header  = Rect::new(main.origin.x + inner, main.origin.y + inner,
                                    main.size.x - inner * 2.0, hdr_h);

            round_panel(p, header, 10.0, win11::surface2(), win11::stroke());

            let col_name = header.origin.x + 14.0;
            let col_date = header.origin.x + header.size.x * 0.56;
            let col_type = header.origin.x + header.size.x * 0.75;
            let col_size = header.origin.x + header.size.x * 0.90;

            for (col, label) in [(col_name, "Name"), (col_date, "Date modified"), (col_type, "Type"), (col_size, "Size")] {
                push_text(p, self.font, Vec2::new(col, header.origin.y + 11.0), label, 12.5, win11::text_faint());
            }

            let list = Rect::new(
                main.origin.x + inner,
                header.origin.y + hdr_h + 10.0,
                main.size.x - inner * 2.0,
                main.size.y - inner * 2.0 - hdr_h - 10.0,
            );
            round_panel(p, list, 12.0, win11::surface2(), win11::stroke());

            // Scrollbar
            let rail = Rect::new(list.origin.x + list.size.x - 10.0, list.origin.y + 8.0, 4.0, list.size.y - 16.0);
            p.fill_rounded_rect(rail, 2.0, Paint::Solid(Color::from_straight(1.0, 1.0, 1.0, 0.06)), None);
            let thumb_h = (rail.size.y * 0.28).max(28.0);
            let thumb   = Rect::new(rail.origin.x, rail.origin.y + (rail.size.y - thumb_h) * 0.20, rail.size.x, thumb_h);
            p.fill_rounded_rect(thumb, 2.0, Paint::Solid(Color::from_straight(1.0, 1.0, 1.0, 0.16)), None);

            p.push_clip(list);

            let row_h = 40.0;
            let mut ry = list.origin.y + 8.0 - self.scroll_y;
            for (i, row) in FILES.iter().enumerate() {
                if ry + row_h < list.origin.y { ry += row_h; continue; }
                if ry > list.origin.y + list.size.y { break; }

                let rr = Rect::new(list.origin.x + 8.0, ry, list.size.x - 16.0, row_h - 6.0);
                if i == self.selected { list_row_bg(p, rr, true); }

                let ic = Rect::new(rr.origin.x + 10.0, rr.origin.y + 8.0, 24.0, 24.0);
                icon_glyph(p, self.font, ic, Color::from_straight(1.0, 1.0, 1.0, 0.03), win11::stroke(),
                    if row.kind == "Folder" { "📁" } else { "📄" });

                push_text(p, self.font, Vec2::new(rr.origin.x + 44.0, rr.origin.y + 11.0), &ellipsize(row.name, 34), 13.8, win11::text());
                push_text(p, self.font, Vec2::new(col_date, rr.origin.y + 11.0), row.date, 13.0, win11::text_dim());
                push_text(p, self.font, Vec2::new(col_type, rr.origin.y + 11.0), row.kind, 13.0, win11::text_dim());
                push_text(p, self.font, Vec2::new(col_size, rr.origin.y + 11.0), row.size, 13.0, win11::text_dim());

                // Divider
                p.fill_rect(Rect::new(rr.origin.x + 8.0, rr.origin.y + rr.size.y + 2.0, rr.size.x - 16.0, 1.0),
                    Color::from_straight(1.0, 1.0, 1.0, 0.04));

                ry += row_h;
            }

            p.pop_clip();

            // Footer watermark
            push_text(p, self.font,
                Vec2::new(list.origin.x + 14.0, list.origin.y + list.size.y - 26.0),
                "Mock Explorer UI — marduk-ui framework — zero engine imports in app code",
                11.5, win11::text_faint());
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        if let UiEvent::Click { pos } = event {
            // Handle row selection clicks inside the file list area.
            let pad     = 16.0_f32;
            let shell   = Rect::new(pad, pad, rect.size.x - pad * 2.0, rect.size.y - pad * 2.0);
            let title_h = 44.0;
            let cmd_h   = 54.0;
            let crumb_h = 46.0;
            let sidebar_w = 246.0;
            let content_y = shell.origin.y + title_h + cmd_h + crumb_h;
            let main = Rect::new(shell.origin.x + sidebar_w, content_y,
                                 shell.size.x - sidebar_w, shell.size.y - (title_h + cmd_h + crumb_h));

            let inner  = 16.0;
            let hdr_h  = 38.0;
            let list_y = main.origin.y + inner + hdr_h + 10.0;
            let list_h = main.size.y - inner * 2.0 - hdr_h - 10.0;
            let list   = Rect::new(main.origin.x + inner, list_y, main.size.x - inner * 2.0, list_h);

            if list.contains(*pos) {
                let row_h = 40.0;
                let rel_y = pos.y - list.origin.y - 8.0 + self.scroll_y;
                let i = (rel_y / row_h) as usize;
                if i < FILES.len() {
                    self.selected = i;
                    return EventResult::Consumed;
                }
            }
        }
        EventResult::Ignored
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

fn load_font() -> Vec<u8> {
    [
        "/usr/share/fonts/TTF/SegoeUI.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/OpenSans-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    ]
    .iter()
    .find_map(|p| std::fs::read(p).ok())
    .unwrap_or_default()
}

fn main() {
    Application::new()
        .title("Marduk Studio")
        .size(1280.0, 720.0)
        .font("body", load_font())
        .on_event("window_close", || std::process::exit(0))
        .run_widget(|fonts| StudioRoot::new(fonts.get("body")).into())
}
