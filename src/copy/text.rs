use crate::error::{Error, Result};

/// Encoder for text COPY IN format.
///
/// Text COPY format: tab-separated fields, newline-separated rows.
/// Special values: `\N` for NULL, backslash escaping for special chars.
///
/// # Example
///
/// ```rust
/// use sentinel_driver::copy::text::TextCopyEncoder;
///
/// let mut encoder = TextCopyEncoder::new();
/// encoder.add_row(&[Some("42"), Some("hello world")]);
/// encoder.add_row(&[Some("7"), None]); // NULL value
///
/// let data = encoder.finish();
/// ```
pub struct TextCopyEncoder {
    buf: String,
}

impl TextCopyEncoder {
    pub fn new() -> Self {
        Self {
            buf: String::with_capacity(8192),
        }
    }

    /// Add a row with the given field values.
    ///
    /// `None` represents NULL (encoded as `\N`).
    /// Values are tab-separated, rows are newline-separated.
    pub fn add_row(&mut self, fields: &[Option<&str>]) {
        for (i, field) in fields.iter().enumerate() {
            if i > 0 {
                self.buf.push('\t');
            }
            match field {
                Some(val) => escape_text_value(&mut self.buf, val),
                None => self.buf.push_str("\\N"),
            }
        }
        self.buf.push('\n');
    }

    /// Finish encoding and return the text COPY data.
    pub fn finish(self) -> Vec<u8> {
        self.buf.into_bytes()
    }

    /// Get the current buffer size.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl Default for TextCopyEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape a text value for COPY format.
///
/// Backslash, tab, newline, and carriage return need escaping.
fn escape_text_value(buf: &mut String, val: &str) {
    for ch in val.chars() {
        match ch {
            '\\' => buf.push_str("\\\\"),
            '\t' => buf.push_str("\\t"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            other => buf.push(other),
        }
    }
}

/// Decoder for text COPY OUT format.
///
/// Parses tab-separated, newline-separated text data.
pub struct TextCopyDecoder;

impl TextCopyDecoder {
    /// Parse a single line of text COPY data into field values.
    ///
    /// Returns `None` for NULL fields (`\N`).
    pub fn parse_line(line: &str) -> Result<Vec<Option<String>>> {
        let mut fields = Vec::new();

        for raw_field in line.split('\t') {
            if raw_field == "\\N" {
                fields.push(None);
            } else {
                fields.push(Some(unescape_text_value(raw_field)?));
            }
        }

        Ok(fields)
    }

    /// Parse multiple lines of text COPY data.
    pub fn parse_all(data: &str) -> Result<Vec<Vec<Option<String>>>> {
        let mut rows = Vec::new();

        for line in data.lines() {
            if line.is_empty() {
                continue;
            }
            rows.push(Self::parse_line(line)?);
        }

        Ok(rows)
    }
}

/// Unescape a text COPY field value.
fn unescape_text_value(val: &str) -> Result<String> {
    let mut result = String::with_capacity(val.len());
    let mut chars = val.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') | None => result.push('\\'),
                Some('t') => result.push('\t'),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('N') => {
                    // Should not happen here (handled at field level)
                    return Err(Error::Copy("unexpected \\N inside field".into()));
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple_row() {
        let mut encoder = TextCopyEncoder::new();
        encoder.add_row(&[Some("42"), Some("hello")]);
        let data = String::from_utf8(encoder.finish()).unwrap();
        assert_eq!(data, "42\thello\n");
    }

    #[test]
    fn test_encode_null() {
        let mut encoder = TextCopyEncoder::new();
        encoder.add_row(&[Some("1"), None, Some("three")]);
        let data = String::from_utf8(encoder.finish()).unwrap();
        assert_eq!(data, "1\t\\N\tthree\n");
    }

    #[test]
    fn test_encode_escape_special_chars() {
        let mut encoder = TextCopyEncoder::new();
        encoder.add_row(&[
            Some("line1\nline2"),
            Some("col1\tcol2"),
            Some("back\\slash"),
        ]);
        let data = String::from_utf8(encoder.finish()).unwrap();
        assert_eq!(data, "line1\\nline2\tcol1\\tcol2\tback\\\\slash\n");
    }

    #[test]
    fn test_decode_simple_line() {
        let fields = TextCopyDecoder::parse_line("42\thello").unwrap();
        assert_eq!(
            fields,
            vec![Some("42".to_string()), Some("hello".to_string())]
        );
    }

    #[test]
    fn test_decode_null() {
        let fields = TextCopyDecoder::parse_line("1\t\\N\tthree").unwrap();
        assert_eq!(
            fields,
            vec![Some("1".to_string()), None, Some("three".to_string()),]
        );
    }

    #[test]
    fn test_decode_escaped_chars() {
        let fields =
            TextCopyDecoder::parse_line("line1\\nline2\tcol1\\tcol2\tback\\\\slash").unwrap();
        assert_eq!(
            fields,
            vec![
                Some("line1\nline2".to_string()),
                Some("col1\tcol2".to_string()),
                Some("back\\slash".to_string()),
            ]
        );
    }

    #[test]
    fn test_roundtrip_text() {
        let mut encoder = TextCopyEncoder::new();
        encoder.add_row(&[Some("hello\tworld"), None, Some("back\\slash\nnewline")]);

        let data = String::from_utf8(encoder.finish()).unwrap();
        let rows = TextCopyDecoder::parse_all(&data).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], Some("hello\tworld".to_string()));
        assert_eq!(rows[0][1], None);
        assert_eq!(rows[0][2], Some("back\\slash\nnewline".to_string()));
    }

    #[test]
    fn test_multiple_rows() {
        let mut encoder = TextCopyEncoder::new();
        encoder.add_row(&[Some("1"), Some("Alice")]);
        encoder.add_row(&[Some("2"), Some("Bob")]);
        encoder.add_row(&[Some("3"), Some("Charlie")]);

        let data = String::from_utf8(encoder.finish()).unwrap();
        let rows = TextCopyDecoder::parse_all(&data).unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2][1], Some("Charlie".to_string()));
    }
}
