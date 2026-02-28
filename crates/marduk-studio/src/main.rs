use marduk_ui::Application;

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
        .size(620.0, 500.0)
        .zoom(1.0)   // Ctrl+Scroll to adjust at runtime
        .font("body", load_font())
        // ── components ────────────────────────────────────────────────────
        .component("Header",   include_str!("../ui/header.mkml"))
        .component("Fleet",    include_str!("../ui/components/fleet.mkml"))
        .component("Dispatch", include_str!("../ui/components/dispatch.mkml"))
        // ── scroll ────────────────────────────────────────────────────────
        .on_event("main_scroll", || {})
        // ── dispatch ──────────────────────────────────────────────────────
        .on_event("dispatch_note_changed", || {})
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
        // ── slider ────────────────────────────────────────────────────────
        .on_event("fuel_threshold_changed", || println!("  [CFG] Fuel warning threshold updated."))
        // ── toggles ───────────────────────────────────────────────────────
        .on_event("gps_toggled",            || println!("  [SYS] GPS tracking toggled."))
        .on_event("sms_toggled",            || println!("  [SYS] SMS alerts toggled."))
        // ── checkbox ──────────────────────────────────────────────────────
        .on_event("load_photo_toggled",     || println!("  [CFG] Load photo requirement toggled."))
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
