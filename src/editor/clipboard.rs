use arboard::Clipboard;

pub fn copy_to_clipboard(text: &str) {
    match Clipboard::new() {
        Ok(mut cb) => {
            if let Err(e) = cb.set_text(text) {
                tracing::warn!("clipboard set failed: {}", e);
            }
        }
        Err(e) => tracing::warn!("clipboard init failed: {}", e),
    }
}

pub fn paste_from_clipboard() -> Option<String> {
    match Clipboard::new() {
        Ok(mut cb) => match cb.get_text() {
            Ok(text) => Some(text),
            Err(e) => {
                tracing::warn!("clipboard get failed: {}", e);
                None
            }
        },
        Err(e) => {
            tracing::warn!("clipboard init failed: {}", e);
            None
        }
    }
}
