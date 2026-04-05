use std::fmt::Write;

const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

static ESCAPE_LUT: [u8; 128] = {
    let mut lut = [0u8; 128];
    // default to 1 (generic \xHH escape) for control chars 0..31
    let mut i = 0;
    while i < 32 {
        lut[i] = 1;
        i += 1;
    }
    lut[0x7F] = 1; // DEL

    // named escapes
    lut[b'"' as usize] = 2;
    lut[b'\\' as usize] = 3;
    lut[b'\n' as usize] = 4;
    lut[b'\t' as usize] = 5;
    lut[0x07] = 6; // \a
    lut[0x08] = 7; // \b
    lut[0x0C] = 8; // \f
    lut[0x0B] = 9; // \v
    lut[b'\r' as usize] = 10;
    lut
};

pub fn append_luau(out: &mut String, source: &str) {
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii() {
            let b = ch as u8;
            match ESCAPE_LUT[b as usize] {
                0 => out.push(ch), // Common case: Passthrough
                2 => out.push_str("\\\""),
                3 => out.push_str("\\\\"),
                4 => out.push_str("\\n"),
                5 => out.push_str("\\t"),
                6 => out.push_str("\\a"),
                7 => out.push_str("\\b"),
                8 => out.push_str("\\f"),
                9 => out.push_str("\\v"),
                10 => {
                    // Normalize \r and \r\n to \n
                    out.push_str("\\n");
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                }
                _ => {
                    // Case 1: Generic control char \xHH
                    out.push_str("\\x");
                    out.push(HEX_DIGITS[(b >> 4) as usize] as char);
                    out.push(HEX_DIGITS[(b & 0xF) as usize] as char);
                }
            }
        } else {
            // Non-ASCII: Use Luau's unicode escape \u{...}
            write!(out, "\\u{{{:X}}}", ch as u32).unwrap();
        }
    }
}

pub fn append_luau_string(out: &mut String, source: &str) {
    out.push('"');
    append_luau(out, source);
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escape(s: &str) -> String {
        let mut out = String::new();
        append_luau(&mut out, s);
        out
    }

    fn escape_str(s: &str) -> String {
        let mut out = String::new();
        append_luau_string(&mut out, s);
        out
    }

    #[test]
    fn plain_ascii() {
        assert_eq!(escape("hello world"), "hello world");
        assert_eq!(escape("abc123!@#"), "abc123!@#");
    }

    #[test]
    fn escape_quotes_and_backslash() {
        assert_eq!(escape("say \"hi\""), r#"say \"hi\""#);
        assert_eq!(escape(r"a\b"), r"a\\b");
    }

    #[test]
    fn escape_newlines_and_tabs() {
        assert_eq!(escape("\n"), "\\n");
        assert_eq!(escape("\t"), "\\t");
        assert_eq!(escape("\r\n"), "\\n");
        assert_eq!(escape("\r"), "\\n");
    }

    #[test]
    fn escape_other_named() {
        assert_eq!(escape("\x07"), "\\a");
        assert_eq!(escape("\x08"), "\\b");
        assert_eq!(escape("\x0C"), "\\f");
        assert_eq!(escape("\x0B"), "\\v");
    }

    #[test]
    fn escape_control_chars() {
        assert_eq!(escape("\x01"), "\\x01");
        assert_eq!(escape("\x1F"), "\\x1F");
        assert_eq!(escape("\x7F"), "\\x7F");
    }

    #[test]
    fn escape_non_ascii() {
        assert_eq!(escape("é"), "\\u{E9}");
        assert_eq!(escape("→"), "\\u{2192}");
        assert_eq!(escape("😀"), "\\u{1F600}");
    }

    #[test]
    fn string_wraps_in_quotes() {
        assert_eq!(escape_str("hi"), "\"hi\"");
        assert_eq!(escape_str("a\"b"), "\"a\\\"b\"");
    }
}
