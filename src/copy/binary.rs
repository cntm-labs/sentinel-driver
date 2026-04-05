use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};

/// Binary COPY header: `PGCOPY\n\377\r\n\0` + flags(4) + extension_len(4)
const BINARY_HEADER: &[u8] = b"PGCOPY\n\xff\r\n\0";
const HEADER_FLAGS: i32 = 0;
const HEADER_EXTENSION_LEN: i32 = 0;

/// Binary COPY trailer: field count = -1
const BINARY_TRAILER_FIELD_COUNT: i16 = -1;

/// Encoder for binary COPY IN format.
///
/// Builds a buffer containing the binary header, tuple data, and trailer.
///
/// # Example
///
/// ```rust
/// use sentinel_driver::copy::binary::BinaryCopyEncoder;
///
/// let mut encoder = BinaryCopyEncoder::new();
///
/// // Write a row with two columns: int4(42) and text("hello")
/// encoder.begin_row(2);
/// encoder.write_field(&42i32.to_be_bytes());
/// encoder.write_field(b"hello");
///
/// // Write another row with a NULL second column
/// encoder.begin_row(2);
/// encoder.write_field(&7i32.to_be_bytes());
/// encoder.write_null();
///
/// let data = encoder.finish();
/// // data can be sent via CopyIn::write_raw()
/// ```
pub struct BinaryCopyEncoder {
    buf: BytesMut,
    header_written: bool,
}

impl BinaryCopyEncoder {
    pub fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(8192),
            header_written: false,
        }
    }

    fn ensure_header(&mut self) {
        if !self.header_written {
            self.buf.put_slice(BINARY_HEADER);
            self.buf.put_i32(HEADER_FLAGS);
            self.buf.put_i32(HEADER_EXTENSION_LEN);
            self.header_written = true;
        }
    }

    /// Begin a new row with the given number of fields.
    pub fn begin_row(&mut self, field_count: i16) {
        self.ensure_header();
        self.buf.put_i16(field_count);
    }

    /// Write a non-NULL field value (already in binary PG format).
    pub fn write_field(&mut self, data: &[u8]) {
        self.buf.put_i32(data.len() as i32);
        self.buf.put_slice(data);
    }

    /// Write a NULL field.
    pub fn write_null(&mut self) {
        self.buf.put_i32(-1);
    }

    /// Finish encoding and return the complete binary COPY data.
    ///
    /// Appends the trailer (field_count = -1).
    pub fn finish(mut self) -> Vec<u8> {
        self.ensure_header();
        self.buf.put_i16(BINARY_TRAILER_FIELD_COUNT);
        self.buf.to_vec()
    }

    /// Get the current buffer size.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl Default for BinaryCopyEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Decoder for binary COPY OUT format.
///
/// Parses binary COPY data received from the server.
pub struct BinaryCopyDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    header_parsed: bool,
}

impl<'a> BinaryCopyDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            header_parsed: false,
        }
    }

    /// Parse the binary COPY header. Must be called before reading rows.
    pub fn parse_header(&mut self) -> Result<()> {
        if self.data.len() < BINARY_HEADER.len() + 8 {
            return Err(Error::Copy("binary COPY data too short for header".into()));
        }

        if &self.data[..BINARY_HEADER.len()] != BINARY_HEADER {
            return Err(Error::Copy("invalid binary COPY header".into()));
        }

        self.pos = BINARY_HEADER.len();

        // flags
        let _flags = read_i32(self.data, self.pos);
        self.pos += 4;

        // extension area length
        let ext_len = read_i32(self.data, self.pos) as usize;
        self.pos += 4;

        // skip extension area
        self.pos += ext_len;

        self.header_parsed = true;
        Ok(())
    }

    /// Read the next row. Returns `None` at the trailer.
    ///
    /// Each field is returned as `Option<&[u8]>` (None = NULL).
    pub fn next_row(&mut self) -> Result<Option<Vec<Option<&'a [u8]>>>> {
        if !self.header_parsed {
            self.parse_header()?;
        }

        if self.pos + 2 > self.data.len() {
            return Ok(None);
        }

        let field_count = read_i16(self.data, self.pos);
        self.pos += 2;

        // Trailer: field_count == -1
        if field_count == BINARY_TRAILER_FIELD_COUNT {
            return Ok(None);
        }

        if field_count < 0 {
            return Err(Error::Copy(format!("invalid field count: {field_count}")));
        }

        let mut fields = Vec::with_capacity(field_count as usize);

        for _ in 0..field_count {
            if self.pos + 4 > self.data.len() {
                return Err(Error::Copy("truncated binary COPY row".into()));
            }

            let len = read_i32(self.data, self.pos);
            self.pos += 4;

            if len == -1 {
                fields.push(None); // NULL
            } else if len < 0 {
                return Err(Error::Copy(format!("invalid field length: {len}")));
            } else {
                let len = len as usize;
                if self.pos + len > self.data.len() {
                    return Err(Error::Copy("truncated binary COPY field".into()));
                }
                fields.push(Some(&self.data[self.pos..self.pos + len]));
                self.pos += len;
            }
        }

        Ok(Some(fields))
    }
}

fn read_i32(buf: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

fn read_i16(buf: &[u8], offset: usize) -> i16 {
    i16::from_be_bytes([buf[offset], buf[offset + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut encoder = BinaryCopyEncoder::new();

        // Row 1: int4(42), text("hello")
        encoder.begin_row(2);
        encoder.write_field(&42i32.to_be_bytes());
        encoder.write_field(b"hello");

        // Row 2: int4(7), NULL
        encoder.begin_row(2);
        encoder.write_field(&7i32.to_be_bytes());
        encoder.write_null();

        let data = encoder.finish();

        // Decode
        let mut decoder = BinaryCopyDecoder::new(&data);

        let row1 = decoder.next_row().unwrap().unwrap();
        assert_eq!(row1.len(), 2);
        assert_eq!(row1[0].unwrap(), &42i32.to_be_bytes());
        assert_eq!(row1[1].unwrap(), b"hello");

        let row2 = decoder.next_row().unwrap().unwrap();
        assert_eq!(row2.len(), 2);
        assert_eq!(row2[0].unwrap(), &7i32.to_be_bytes());
        assert!(row2[1].is_none()); // NULL

        // Trailer
        assert!(decoder.next_row().unwrap().is_none());
    }

    #[test]
    fn test_encode_empty() {
        let encoder = BinaryCopyEncoder::new();
        let data = encoder.finish();

        // Should have header + trailer
        let mut decoder = BinaryCopyDecoder::new(&data);
        assert!(decoder.next_row().unwrap().is_none());
    }

    #[test]
    fn test_header_validation() {
        let mut decoder = BinaryCopyDecoder::new(b"not a valid header at all!!!");
        assert!(decoder.parse_header().is_err());
    }

    #[test]
    fn test_encoder_size() {
        let mut encoder = BinaryCopyEncoder::new();

        // Header: 11 + 4 + 4 = 19 bytes
        encoder.begin_row(1);
        // Row: 2 (field count) + 4 (field len) + 4 (data) = 10
        encoder.write_field(&42i32.to_be_bytes());

        let data = encoder.finish();
        // 19 (header) + 10 (row) + 2 (trailer) = 31
        assert_eq!(data.len(), 31);
    }

    #[test]
    fn test_multiple_rows() {
        let mut encoder = BinaryCopyEncoder::new();

        for i in 0..100 {
            encoder.begin_row(1);
            encoder.write_field(&(i as i32).to_be_bytes());
        }

        let data = encoder.finish();
        let mut decoder = BinaryCopyDecoder::new(&data);

        let mut count = 0;
        while let Some(row) = decoder.next_row().unwrap() {
            let val = i32::from_be_bytes(row[0].unwrap().try_into().unwrap());
            assert_eq!(val, count);
            count += 1;
        }
        assert_eq!(count, 100);
    }
}
