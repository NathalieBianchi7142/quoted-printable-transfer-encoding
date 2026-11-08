//! Quoted-printable transfer encoding (RFC 2045).
//!
//! This crate implements the quoted-printable encoding and decoding as
//! specified in RFC 2045 section 6.7. The encoding is used to represent
//! arbitrary 8-bit data in a 7-bit ASCII channel, such as email.
//!
//! The implementation is strict about decoding: malformed input returns an
//! error rather than being silently ignored. This makes it suitable for
//! applications that need to detect corruption.
//!
//! # Examples
//!
//! ```
//! use quoted_printable_transfer_encoding::{encode, decode};
//!
//! let data = b"Hello, world!";
//! let encoded = encode(data, 76);
//! assert_eq!(encoded, "Hello, world!");
//!
//! let decoded = decode(&encoded).unwrap();
//! assert_eq!(decoded, data);
//! ```

/// The maximum line length allowed by the encoding, including the trailing
/// soft line break (if any). RFC 2045 recommends 76 characters.
pub const DEFAULT_LINE_LENGTH: usize = 76;

/// Errors that can occur during decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// A '=' character was followed by something other than two hexadecimal
    /// digits or a line break.
    InvalidEscape,
    /// A hexadecimal digit was not a valid hex character.
    InvalidHexDigit(char),
    /// The input ended in the middle of an escape sequence.
    UnexpectedEnd,
    /// A soft line break was not properly terminated.
    InvalidLineBreak,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InvalidEscape => write!(f, "invalid escape sequence"),
            DecodeError::InvalidHexDigit(c) => write!(f, "invalid hex digit: {}", c),
            DecodeError::UnexpectedEnd => write!(f, "unexpected end of input"),
            DecodeError::InvalidLineBreak => write!(f, "invalid line break"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Encodes `input` using quoted-printable encoding, wrapping lines at
/// `line_length` characters.
///
/// The `line_length` parameter is the maximum number of characters per output
/// line, including any soft line break (`=` followed by CRLF). If `line_length`
/// is less than 4, it is clamped to 4 to ensure that at least one character can
/// be encoded per line.
///
/// # Panics
///
/// This function does not panic.
pub fn encode(input: &[u8], line_length: usize) -> String {
    let line_length = line_length.max(4);
    let mut output = String::with_capacity(input.len() * 2);
    let mut line_len = 0;

    for &byte in input {
        // Determine the encoded representation of this byte.
        let encoded = if byte == b'\r' {
            // CR is not allowed in the body; encode it.
            "=0D".to_string()
        } else if byte == b'\n' {
            // LF is a line break. We handle it specially below.
            "\n".to_string()
        } else if byte == b'=' {
            "=3D".to_string()
        } else if byte < 32 || byte > 126 {
            // Non-printable or non-ASCII: encode as =XX.
            format!("={:02X}", byte)
        } else {
            // Printable ASCII except '='.
            (byte as char).to_string()
        };

        // If the encoded representation contains a newline, handle it as a
        // hard line break: reset line length and append.
        if encoded == "\n" {
            output.push('\n');
            line_len = 0;
            continue;
        }

        // Check if adding this encoded representation would exceed the line
        // length. We need to account for a possible soft line break if we wrap.
        let encoded_len = encoded.len();
        if line_len + encoded_len > line_length {
            // Wrap: add soft line break and reset line length. A soft line
            // break is `=` followed by CRLF, and must fit within the line.
            // Therefore, if the current line already contains characters,
            // the soft break consumes one character of the line's budget.
            if line_len > 0 {
                output.push_str("=\r\n");
            }
            line_len = 0;
        }

        output.push_str(&encoded);
        line_len += encoded_len;
    }

    output
}

/// Decodes a quoted-printable encoded string into bytes.
///
/// Returns an error if the input contains invalid escape sequences or
/// malformed line breaks.
pub fn decode(input: &str) -> Result<Vec<u8>, DecodeError> {
    let mut output = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'=' {
            // Check for soft line break: '=' followed by CRLF or LF.
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                // Soft line break with LF only.
                i += 2;
                continue;
            }
            if i + 2 < bytes.len() && bytes[i + 1] == b'\r' && bytes[i + 2] == b'\n' {
                // Soft line break with CRLF.
                i += 3;
                continue;
            }
            // Otherwise, expect two hex digits.
            if i + 2 >= bytes.len() {
                return Err(DecodeError::UnexpectedEnd);
            }
            let hi = bytes[i + 1];
            let lo = bytes[i + 2];
            let hi_val = hex_value(hi).ok_or(DecodeError::InvalidHexDigit(hi as char))?;
            let lo_val = hex_value(lo).ok_or(DecodeError::InvalidHexDigit(lo as char))?;
            output.push((hi_val << 4) | lo_val);
            i += 3;
        } else if b == b'\r' {
            // CR must be followed by LF; otherwise it's an error.
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                output.push(b'\r');
                output.push(b'\n');
                i += 2;
            } else {
                return Err(DecodeError::InvalidLineBreak);
            }
        } else {
            output.push(b);
            i += 1;
        }
    }

    Ok(output)
}

/// Converts a single ASCII byte to its hexadecimal value.
fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty() {
        assert_eq!(encode(b"", 76), "");
    }

    #[test]
    fn encode_plain_ascii() {
        assert_eq!(encode(b"Hello, world!", 76), "Hello, world!");
    }

    #[test]
    fn encode_equals() {
        assert_eq!(encode(b"a=b", 76), "a=3Db");
    }

    #[test]
    fn encode_non_ascii() {
        assert_eq!(encode(b"\x00\xFF", 76), "=00=FF");
    }

    #[test]
    fn encode_crlf() {
        assert_eq!(encode(b"a\r\nb", 76), "a=0D\nb");
    }

    #[test]
    fn encode_wrapping() {
        // Each character is 1 byte, so 4 characters fit on a line before
        // wrapping.
        assert_eq!(encode(b"abcde", 4), "abcd=\r\ne");
    }

    #[test]
    fn encode_wrapping_with_encoded_bytes() {
        // '=' encodes to 3 bytes, so line length 4 will wrap after one '='.
        assert_eq!(encode(b"=", 4), "=3D");
        assert_eq!(encode(b"==", 4), "=3D=\r\n=3D");
    }

    #[test]
    fn decode_empty() {
        assert_eq!(decode("").unwrap(), b"");
    }

    #[test]
    fn decode_plain_ascii() {
        assert_eq!(decode("Hello, world!").unwrap(), b"Hello, world!");
    }

    #[test]
    fn decode_escaped() {
        assert_eq!(decode("a=3Db").unwrap(), b"a=b");
    }

    #[test]
    fn decode_non_ascii() {
        assert_eq!(decode("=00=FF").unwrap(), b"\x00\xFF");
    }

    #[test]
    fn decode_soft_line_break_crlf() {
        assert_eq!(decode("abc=\r\ndef").unwrap(), b"abcdef");
    }

    #[test]
    fn decode_soft_line_break_lf() {
        assert_eq!(decode("abc=\ndef").unwrap(), b"abcdef");
    }

    #[test]
    fn decode_hard_line_break() {
        assert_eq!(decode("abc\r\ndef").unwrap(), b"abc\r\ndef");
    }

    #[test]
    fn decode_invalid_escape() {
        assert_eq!(decode("=XY"), Err(DecodeError::InvalidHexDigit('X')));
    }

    #[test]
    fn decode_unexpected_end() {
        assert_eq!(decode("=A"), Err(DecodeError::UnexpectedEnd));
    }

    #[test]
    fn decode_invalid_line_break() {
        assert_eq!(decode("abc\rdef"), Err(DecodeError::InvalidLineBreak));
    }

    #[test]
    fn roundtrip() {
        let original = b"Hello, \x00\xFF world!\r\n";
        let encoded = encode(original, 76);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
}
