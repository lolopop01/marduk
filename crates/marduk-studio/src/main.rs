use marduk_ui::{Application, WindowMode};

fn main() {
    println!();
    println!("  ╔══════════════════════════════════════════╗");
    println!("  ║     REDLINE LOGISTICS — OPS CONSOLE      ║");
    println!("  ║   Depot YUL-WEST  ·  marduk-ui v0.1      ║");
    println!("  ╠══════════════════════════════════════════╣");
    println!("  ║  Fleet: 4 units  (1 breakdown, 1 warn)   ║");
    println!("  ║  Warehouse: 94%  ·  2 orders overdue     ║");
    println!("  ║  Awaiting dispatcher input...            ║");
    println!("  ╚══════════════════════════════════════════╝");
    println!();

    Application::new()
        .title("Redline Logistics — Depot YUL-WEST")
        .size(800.0, 600.0)
        .zoom(1.0)
        .window_mode(WindowMode::Fullscreen)
        .font("body", load_font())
        // ── images ────────────────────────────────────────────────────────
        .image("truck_icon", truck_icon_svg())
        // ── components ────────────────────────────────────────────────────
        .component("Header",  include_str!("../ui/header.mkml"))
        .component("Fleet",   include_str!("../ui/components/fleet.mkml"))
        .component("Dispatch",include_str!("../ui/components/dispatch.mkml"))
        .component("Routing", include_str!("../ui/components/routing.mkml"))
        .component("Tools",   include_str!("../ui/components/tools.mkml"))
        // ── OPS tab ───────────────────────────────────────────────────────
        .on_event("main_scroll",              || {})
        .on_event("main_tab_changed",         || {})
        .on_event("dispatch_note_changed",    || {})
        .on_event("send_dispatch_note", || {
            println!();
            println!("  [DISPATCH] Note queued for transmission.");
            println!("  Status   ... SENT to active drivers");
            println!();
        })
        .on_event_state("clear_dispatch_note", |state| {
            state.clear("dispatch_note");
            println!("  [DISPATCH] Note cleared.");
        })
        .on_event("fuel_threshold_changed",   || println!("  [CFG] Fuel warning threshold updated."))
        .on_event("gps_toggled",              || println!("  [SYS] GPS tracking toggled."))
        .on_event("sms_toggled",              || println!("  [SYS] SMS alerts toggled."))
        .on_event("load_photo_toggled",       || println!("  [CFG] Load photo requirement toggled."))
        // ── ROUTING tab (Splitter) ─────────────────────────────────────────
        .on_event("route_split_changed",      || {})
        // ── TOOLS tab (Combobox / NumberInput / Modal) ─────────────────────
        .on_event("tools_scroll",             || {})
        .on_event("driver_assigned", || println!("  [ASSIGN] Driver assigned to TRK-004."))
        .on_event("priority_changed",         || println!("  [ASSIGN] Route priority updated."))
        .on_event("load_limit_changed",       || println!("  [LIMITS] Max load updated."))
        .on_event("speed_limit_changed",      || println!("  [LIMITS] Speed limit updated."))
        .on_event("rest_interval_changed",    || println!("  [LIMITS] Rest interval updated."))
        // ── Modal ──────────────────────────────────────────────────────────
        .on_event_state("open_emergency", |state| {
            state.set_bool("emergency_modal", true);
        })
        .on_event_state("dismiss_emergency", |state| {
            state.set_bool("emergency_modal", false);
        })
        .on_event_state("confirm_emergency", |state| {
            state.set_bool("emergency_modal", false);
            println!();
            println!("  [EMERGENCY] *** HALT SIGNAL SENT TO ALL ACTIVE ROUTES ***");
            println!("  [EMERGENCY] TRK-001 TRK-002 TRK-003 — drivers notified.");
            println!();
        })
        .run(include_str!("../ui/main.mkml"))
}

fn load_font() -> Vec<u8> {
    [
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    ]
    .iter()
    .find_map(|p| std::fs::read(p).ok())
    .unwrap_or_default()
}

/// A simple SVG truck icon for testing image rendering.
fn truck_icon_svg() -> Vec<u8> {
    br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
      <rect x="2" y="22" width="40" height="26" rx="3" fill="#c8a84b"/>
      <path d="M42 28h10l8 12v8h-18V28z" fill="#a08030"/>
      <circle cx="14" cy="52" r="7" fill="#1a1a1a"/>
      <circle cx="14" cy="52" r="3" fill="#555"/>
      <circle cx="50" cy="52" r="7" fill="#1a1a1a"/>
      <circle cx="50" cy="52" r="3" fill="#555"/>
      <rect x="6" y="28" width="18" height="12" rx="2" fill="#7ec8e3" opacity="0.8"/>
    </svg>"##.to_vec()
}
