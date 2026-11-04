//! Terminal paste follows the reference xterm clipboard protocol.
pub const MAX_INPUT_BYTES: usize = 1024 * 1024;
/// Normalize LF/CRLF to CR, preserving lone CR and exact Unicode text. Programs
/// that enable bracketed paste receive one framed paste rather than key events.
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if bracketed {
        let mut bytes = Vec::with_capacity(text.len() + 12);
        bytes.extend_from_slice(b"\x1b[200~");
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(b"\x1b[201~");
        bytes
    } else {
        text.into_bytes()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paste_matches_xterm_line_endings_and_preserves_unicode_and_literal_characters() {
        let text = "first\r\n界🌍\nlast\rtab\t'\\$";
        assert_eq!(
            paste_bytes(text, false),
            "first\r界🌍\rlast\rtab\t'\\$".as_bytes()
        );
        assert_eq!(
            paste_bytes(text, true),
            "\x1b[200~first\r界🌍\rlast\rtab\t'\\$\x1b[201~".as_bytes()
        );
    }
    #[test]
    fn paste_framing_tracks_the_terminal_mode() {
        let mut emulator = crate::emulator::Emulator::new(80, 24);
        assert_eq!(
            paste_bytes("echo 42", emulator.bracketed_paste_mode()),
            b"echo 42"
        );
        emulator.feed(b"\x1b[?2004h");
        assert_eq!(
            paste_bytes("echo 42", emulator.bracketed_paste_mode()),
            b"\x1b[200~echo 42\x1b[201~"
        );
        emulator.feed(b"\x1b[?2004l");
        assert_eq!(
            paste_bytes("echo 42", emulator.bracketed_paste_mode()),
            b"echo 42"
        );
    }
}
