use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};

/// Result of polling for a key: either a key was pressed, or a terminal resize occurred.
pub enum PollResult {
    Key(KeyEvent),
    Resize,
}

/// Poll the tty fd for input, returning a key event or a resize signal.
/// `resize_flag` is an AtomicBool set by a SIGWINCH handler.
pub fn poll_key(reader: &mut std::fs::File, resize_flag: &AtomicBool) -> std::io::Result<PollResult> {
    let fd = reader.as_raw_fd();
    loop {
        // Check if a resize happened
        if resize_flag.swap(false, Ordering::Relaxed) {
            return Ok(PollResult::Resize);
        }

        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        // Poll with 100ms timeout so we can check the resize flag periodically
        let ret = unsafe { libc::poll(&mut pfd as *mut _, 1, 100) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                // EINTR from signal — check resize flag on next iteration
                continue;
            }
            return Err(err);
        }
        if ret == 0 {
            // Timeout — loop back to check resize flag
            continue;
        }
        // Data available — read the key
        return Ok(PollResult::Key(read_key(reader)?));
    }
}

/// Read a key event directly from a file (typically /dev/tty).
/// Bypasses crossterm's mio-based event system which fails in zle widget contexts.
pub fn read_key(reader: &mut impl Read) -> std::io::Result<KeyEvent> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;

    match buf[0] {
        // Ctrl+A..Ctrl+Z (except special ones below)
        1 => key(KeyCode::Char('a'), KeyModifiers::CONTROL),
        2 => key(KeyCode::Char('b'), KeyModifiers::CONTROL),
        3 => key(KeyCode::Char('c'), KeyModifiers::CONTROL),
        4 => key(KeyCode::Char('d'), KeyModifiers::CONTROL),
        5 => key(KeyCode::Char('e'), KeyModifiers::CONTROL),
        6 => key(KeyCode::Char('f'), KeyModifiers::CONTROL),
        9 => key(KeyCode::Tab, KeyModifiers::NONE),
        13 => key(KeyCode::Enter, KeyModifiers::NONE),
        14 => key(KeyCode::Char('n'), KeyModifiers::CONTROL),
        16 => key(KeyCode::Char('p'), KeyModifiers::CONTROL),
        19 => key(KeyCode::Char('s'), KeyModifiers::CONTROL),
        21 => key(KeyCode::Char('u'), KeyModifiers::CONTROL),
        23 => key(KeyCode::Char('w'), KeyModifiers::CONTROL),
        25 => key(KeyCode::Char('y'), KeyModifiers::CONTROL),
        27 => read_escape(reader),
        127 => key(KeyCode::Backspace, KeyModifiers::NONE),
        // UTF-8 multibyte
        b if b >= 0x80 => read_utf8(reader, b),
        // Regular ASCII char
        b => key(KeyCode::Char(b as char), KeyModifiers::NONE),
    }
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> std::io::Result<KeyEvent> {
    Ok(KeyEvent::new(code, modifiers))
}

fn read_escape(reader: &mut impl Read) -> std::io::Result<KeyEvent> {
    let mut buf = [0u8; 1];

    // Try to read next byte; if nothing follows ESC quickly, it's just Esc
    // Use a non-blocking approach: set a short timeout or just try reading
    // Since we're in raw mode, the next byte should be available immediately for escape sequences
    if reader.read(&mut buf)? == 0 {
        return key(KeyCode::Esc, KeyModifiers::NONE);
    }

    match buf[0] {
        b'[' => read_csi(reader),
        b'O' => {
            // SS3 sequences (some terminals send these for F1-F4, etc.)
            let mut b = [0u8; 1];
            if reader.read(&mut b)? == 0 {
                return key(KeyCode::Esc, KeyModifiers::NONE);
            }
            match b[0] {
                b'A' => key(KeyCode::Up, KeyModifiers::NONE),
                b'B' => key(KeyCode::Down, KeyModifiers::NONE),
                b'C' => key(KeyCode::Right, KeyModifiers::NONE),
                b'D' => key(KeyCode::Left, KeyModifiers::NONE),
                b'P' => key(KeyCode::F(1), KeyModifiers::NONE),
                b'Q' => key(KeyCode::F(2), KeyModifiers::NONE),
                b'R' => key(KeyCode::F(3), KeyModifiers::NONE),
                b'S' => key(KeyCode::F(4), KeyModifiers::NONE),
                _ => key(KeyCode::Esc, KeyModifiers::NONE),
            }
        }
        _ => key(KeyCode::Esc, KeyModifiers::NONE),
    }
}

fn read_csi(reader: &mut impl Read) -> std::io::Result<KeyEvent> {
    // Collect parameter bytes (0x30..0x3F) and intermediate bytes (0x20..0x2F),
    // then read the final byte (0x40..0x7E).
    let mut params = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        if reader.read(&mut buf)? == 0 {
            return key(KeyCode::Esc, KeyModifiers::NONE);
        }
        if buf[0] >= 0x40 {
            break; // final byte
        }
        params.push(buf[0]);
    }
    let final_byte = buf[0];
    let param_str: String = params.iter().map(|&b| b as char).collect();

    // Parse semicolon-separated numeric parameters
    let nums: Vec<u32> = param_str
        .split(';')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    match final_byte {
        b'A' => key(KeyCode::Up, KeyModifiers::NONE),
        b'B' => key(KeyCode::Down, KeyModifiers::NONE),
        b'C' => key(KeyCode::Right, KeyModifiers::NONE),
        b'D' => key(KeyCode::Left, KeyModifiers::NONE),
        b'Z' => key(KeyCode::BackTab, KeyModifiers::SHIFT),
        // CSI u: kitty keyboard protocol — ESC[<keycode>;<modifiers>u
        b'u' => {
            let keycode = nums.first().copied().unwrap_or(0);
            let mods = csi_modifiers(nums.get(1).copied().unwrap_or(1));
            match keycode {
                9 => key(KeyCode::Tab, mods),
                13 => key(KeyCode::Enter, mods),
                27 => key(KeyCode::Esc, mods),
                127 => key(KeyCode::Backspace, mods),
                c @ 32..=126 => key(KeyCode::Char((c as u8) as char), mods),
                _ => key(KeyCode::Esc, KeyModifiers::NONE),
            }
        }
        b'H' => key(KeyCode::Home, KeyModifiers::NONE),
        b'F' => key(KeyCode::End, KeyModifiers::NONE),
        b'~' => {
            let code = nums.first().copied().unwrap_or(0);
            let mods = csi_modifiers(nums.get(1).copied().unwrap_or(1));
            match code {
                1 => key(KeyCode::Home, mods),
                3 => key(KeyCode::Delete, mods),
                4 => key(KeyCode::End, mods),
                5 => key(KeyCode::PageUp, mods),
                6 => key(KeyCode::PageDown, mods),
                11 => key(KeyCode::F(1), KeyModifiers::NONE),
                12 => key(KeyCode::F(2), KeyModifiers::NONE),
                13 => key(KeyCode::F(3), KeyModifiers::NONE),
                14 => key(KeyCode::F(4), KeyModifiers::NONE),
                15 => key(KeyCode::F(5), KeyModifiers::NONE),
                17 => key(KeyCode::F(6), KeyModifiers::NONE),
                18 => key(KeyCode::F(7), KeyModifiers::NONE),
                19 => key(KeyCode::F(8), KeyModifiers::NONE),
                20 => key(KeyCode::F(9), KeyModifiers::NONE),
                21 => key(KeyCode::F(10), KeyModifiers::NONE),
                23 => key(KeyCode::F(11), KeyModifiers::NONE),
                24 => key(KeyCode::F(12), KeyModifiers::NONE),
                _ => key(KeyCode::Esc, KeyModifiers::NONE),
            }
        }
        _ => key(KeyCode::Esc, KeyModifiers::NONE),
    }
}

/// Convert CSI modifier parameter to KeyModifiers.
/// CSI convention: modifier = 1 + bitmask (shift=1, alt=2, ctrl=4).
fn csi_modifiers(param: u32) -> KeyModifiers {
    let bits = param.saturating_sub(1);
    let mut mods = KeyModifiers::NONE;
    if bits & 1 != 0 {
        mods |= KeyModifiers::SHIFT;
    }
    if bits & 2 != 0 {
        mods |= KeyModifiers::ALT;
    }
    if bits & 4 != 0 {
        mods |= KeyModifiers::CONTROL;
    }
    mods
}

fn read_utf8(reader: &mut impl Read, first: u8) -> std::io::Result<KeyEvent> {
    let width = if first & 0xE0 == 0xC0 {
        2
    } else if first & 0xF0 == 0xE0 {
        3
    } else if first & 0xF8 == 0xF0 {
        4
    } else {
        return key(KeyCode::Char('?'), KeyModifiers::NONE);
    };

    let mut bytes = vec![first];
    let mut rest = vec![0u8; width - 1];
    reader.read_exact(&mut rest)?;
    bytes.extend_from_slice(&rest);

    let s = String::from_utf8(bytes).unwrap_or_default();
    if let Some(c) = s.chars().next() {
        key(KeyCode::Char(c), KeyModifiers::NONE)
    } else {
        key(KeyCode::Char('?'), KeyModifiers::NONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse(bytes: &[u8]) -> KeyEvent {
        read_key(&mut Cursor::new(bytes)).unwrap()
    }

    #[test]
    fn test_regular_char() {
        let k = parse(b"a");
        assert_eq!(k.code, KeyCode::Char('a'));
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_ctrl_c() {
        let k = parse(&[3]);
        assert_eq!(k.code, KeyCode::Char('c'));
        assert_eq!(k.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_tab() {
        let k = parse(&[9]);
        assert_eq!(k.code, KeyCode::Tab);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_enter() {
        let k = parse(&[13]);
        assert_eq!(k.code, KeyCode::Enter);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_backspace() {
        let k = parse(&[127]);
        assert_eq!(k.code, KeyCode::Backspace);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_escape_arrow_up() {
        let k = parse(&[27, b'[', b'A']);
        assert_eq!(k.code, KeyCode::Up);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_escape_arrow_down() {
        let k = parse(&[27, b'[', b'B']);
        assert_eq!(k.code, KeyCode::Down);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_csi_page_up() {
        let k = parse(&[27, b'[', b'5', b'~']);
        assert_eq!(k.code, KeyCode::PageUp);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_csi_delete() {
        let k = parse(&[27, b'[', b'3', b'~']);
        assert_eq!(k.code, KeyCode::Delete);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_csi_home() {
        let k = parse(&[27, b'[', b'H']);
        assert_eq!(k.code, KeyCode::Home);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_csi_end() {
        let k = parse(&[27, b'[', b'F']);
        assert_eq!(k.code, KeyCode::End);
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_ctrl_s() {
        let k = parse(&[19]);
        assert_eq!(k.code, KeyCode::Char('s'));
        assert_eq!(k.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_csi_f9() {
        // ESC [ 2 0 ~
        let k = parse(&[27, b'[', b'2', b'0', b'~']);
        assert_eq!(k.code, KeyCode::F(9));
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_csi_f12() {
        // ESC [ 2 4 ~
        let k = parse(&[27, b'[', b'2', b'4', b'~']);
        assert_eq!(k.code, KeyCode::F(12));
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_kitty_shift_enter() {
        // ESC [ 1 3 ; 2 u  — keycode 13 (Enter), modifier 2 (shift)
        let k = parse(&[27, b'[', b'1', b'3', b';', b'2', b'u']);
        assert_eq!(k.code, KeyCode::Enter);
        assert_eq!(k.modifiers, KeyModifiers::SHIFT);
    }
}
