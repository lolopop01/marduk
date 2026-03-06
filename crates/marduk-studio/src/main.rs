mod file_explorer;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use marduk_ui::{Application, WindowMode};
use file_explorer::FilePane;

fn main() {
    // File pane lazily initialised on first frame (needs FontId from runtime).
    let event_queue: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let pane_lazy: Rc<RefCell<Option<Rc<RefCell<FilePane>>>>> = Rc::new(RefCell::new(None));

    let eq_drain   = Rc::clone(&event_queue);
    let eq_init    = Rc::clone(&event_queue);
    let pane_slot  = Rc::clone(&pane_lazy);

    Application::new()
        .title("Files")
        .size(1280.0, 800.0)
        .zoom(1.0)
        .window_mode(WindowMode::Fullscreen)
        .font("body", load_font())
        .native_slot("file_explorer", move |fonts| {
            // Print any file-selection events from the previous frame.
            for ev in eq_drain.borrow_mut().drain(..) {
                println!("[FILES] {ev}");
            }

            let font = fonts.get("body").expect("font 'body' not loaded");

            // Lazy-init the pane on first frame.
            let mut guard = pane_slot.borrow_mut();
            if guard.is_none() {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
                *guard = Some(Rc::new(RefCell::new(FilePane::new(
                    PathBuf::from(home),
                    font,
                    Rc::clone(&eq_init),
                ))));
            }
            let pane = guard.as_ref().unwrap().clone();
            drop(guard);

            FilePane::as_element(pane)
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
