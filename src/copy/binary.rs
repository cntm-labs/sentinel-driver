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
