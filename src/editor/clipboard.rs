use arboard::Clipboard;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn copy_to_clipboard(text: &str) {
    let copied_via_command = copy_with_command(text);
    let copied_via_tmux = copy_with_tmux(text);

    if copied_via_command || copied_via_tmux {
        return;
    }

    if copy_with_arboard(text) {
        return;
    }

    tracing::warn!("no clipboard backend succeeded");
}

pub fn paste_from_clipboard() -> Option<String> {
    if let Some(text) = paste_with_command() {
        return Some(text);
    }

    if let Some(text) = paste_with_tmux() {
        return Some(text);
    }

    paste_with_arboard()
}

fn copy_with_command(text: &str) -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && write_to_command("wl-copy", &[], text) {
        return true;
    }

    if std::env::var_os("DISPLAY").is_some() {
        if write_to_command("xclip", &["-selection", "clipboard"], text) {
            return true;
        }
        if write_to_command("xsel", &["--clipboard", "--input"], text) {
            return true;
        }
    }

    false
}

fn paste_with_command() -> Option<String> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        if let Some(text) = read_from_command("wl-paste", &["-n"]) {
            return Some(text);
        }
    }

    if std::env::var_os("DISPLAY").is_some() {
        if let Some(text) = read_from_command("xclip", &["-selection", "clipboard", "-o"]) {
            return Some(text);
        }
        if let Some(text) = read_from_command("xsel", &["--clipboard", "--output"]) {
            return Some(text);
        }
    }

    None
}

fn copy_with_tmux(text: &str) -> bool {
    if std::env::var_os("TMUX").is_none() {
        return false;
    }

    write_to_command("tmux", &["load-buffer", "-w", "-"], text)
}

fn paste_with_tmux() -> Option<String> {
    if std::env::var_os("TMUX").is_none() {
        return None;
    }

    read_from_command("tmux", &["save-buffer", "-"])
}

fn copy_with_arboard(text: &str) -> bool {
    match Clipboard::new() {
        Ok(mut cb) => match cb.set_text(text) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("clipboard set failed: {}", e);
                false
            }
        },
        Err(e) => {
            tracing::warn!("clipboard init failed: {}", e);
            false
        }
    }
}

fn paste_with_arboard() -> Option<String> {
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

fn write_to_command(command: &str, args: &[&str], text: &str) -> bool {
    let mut child = match Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };

    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(text.as_bytes()).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
    }

    child.wait().map(|status| status.success()).unwrap_or(false)
}

fn read_from_command(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout).ok()
}
