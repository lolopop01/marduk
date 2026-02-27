use marduk_ui::Application;

fn main() {
    Application::new()
        .title("Marduk Studio")
        .size(1280.0, 720.0)
        .font("body", load_font())
        .component("Titlebar", include_str!("../ui/titlebar.mkml"))
        .component("Toolbar",  include_str!("../ui/toolbar.mkml"))
        .on_event("window_close",  || std::process::exit(0))
        .on_event("window_minimize", || {})
        .on_event("window_maximize", || {})
        .on_event("nav_back",      || {})
        .on_event("nav_forward",   || {})
        .on_event("new_folder",    || {})
        .on_event("upload_file",   || {})
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
