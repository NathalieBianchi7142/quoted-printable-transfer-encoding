# quoted-printable-transfer-encoding

Encodes and decodes the quoted-printable transfer encoding from RFC 2045 for representing 8-bit data in 7-bit ASCII channels.

## Usage

```rust
use quoted_printable_transfer_encoding::{encode, decode, DecodeError};

let data = b"Hello, world!\r\n";
let encoded = encode(data, 76);
assert_eq!(encoded, "Hello, world!=0D\n");

let decoded = decode(&encoded).unwrap();
assert_eq!(decoded, data);

// Decoding invalid input returns an error.
assert!(decode("=XY").is_err());
```

## Why this library exists

Quoted-printable is used in email to encode arbitrary bytes as printable ASCII. The encoding is simple but has subtle edge cases around line breaks and the `=` character. This library implements the RFC 2045 rules directly, with a strict decoder that rejects malformed input instead of silently ignoring it. The trade-off is that it does not attempt to recover from errors; callers that need lenient decoding should pre-process the input.

## Awkward edge

The encoder treats a literal `\r` as a byte to encode (`=0D`), while `\n` is treated as a hard line break. This matches the RFC's requirement that line breaks in the encoded form are CRLF, but it means that a lone `\r` in the input is preserved as `=0D`. The decoder is strict: a `\r` not followed by `\n` is an error.

## API

- `encode(input: &[u8], line_length: usize) -> String`
- `decode(input: &str) -> Result<Vec<u8>, DecodeError>`
- `DecodeError` variants: `InvalidEscape`, `InvalidHexDigit(char)`, `UnexpectedEnd`, `InvalidLineBreak`
