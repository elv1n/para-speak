use arboard::Clipboard;
use log::debug;
use rdev::{simulate, EventType, Key};
use std::{thread, time};

pub fn get_clipboard() -> String {
    let mut cb = Clipboard::new().ok();
    cb.as_mut()
        .and_then(|c| c.get_text().ok())
        .unwrap_or_default()
}

pub fn set_clipboard(s: &str) {
    if let Ok(mut cb) = Clipboard::new() {
        let _ = cb.set_text(s.to_string());
    }
}

pub fn insert_text_at_cursor(text: &str) {
    debug!("insert_text_at_cursor: {} chars", text.len());

    set_clipboard(text);

    let paste_result = simulate_paste();
    if let Err(e) = paste_result {
        debug!("Failed to simulate paste, user must paste manually: {}", e);
    }
}

fn simulate_paste() -> Result<(), Box<dyn std::error::Error>> {
    let modifier_key = if cfg!(target_os = "macos") {
        Key::MetaLeft
    } else {
        Key::ControlLeft
    };

    let delay = time::Duration::from_millis(50);

    simulate(&EventType::KeyPress(modifier_key))?;
    thread::sleep(delay);

    simulate(&EventType::KeyPress(Key::KeyV))?;
    thread::sleep(delay);

    simulate(&EventType::KeyRelease(Key::KeyV))?;
    thread::sleep(delay);

    simulate(&EventType::KeyRelease(modifier_key))?;
    thread::sleep(delay);

    Ok(())
}
