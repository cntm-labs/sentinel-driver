//! PostgreSQL NUMERIC binary encode/decode for `rust_decimal::Decimal`.
//!
//! PG NUMERIC binary wire format:
//! - ndigits: i16  (number of base-10000 digit groups)
//! - weight:  i16  (exponent of first digit group, in powers of 10000)
//! - sign:    u16  (0x0000 = positive, 0x4000 = negative, 0xC000 = NaN)
//! - dscale:  u16  (number of digits after decimal point)
//! - digits:  [u16; ndigits] (base-10000 digit groups, big-endian)

use bytes::{BufMut, BytesMut};
use rust_decimal::Decimal;

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

const NUMERIC_POS: u16 = 0x0000;
const NUMERIC_NEG: u16 = 0x4000;
const NUMERIC_NAN: u16 = 0xC000;

impl ToSql for Decimal {
    fn oid(&self) -> Oid {
        Oid::NUMERIC
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        if self.is_zero() {
            buf.put_i16(0); // ndigits
            buf.put_i16(0); // weight
            buf.put_u16(NUMERIC_POS); // sign
            buf.put_u16(0); // dscale
            return Ok(());
        }

        let sign = if self.is_sign_negative() {
            NUMERIC_NEG
        } else {
            NUMERIC_POS
        };

        let scale = self.scale() as u16;

        // Get the absolute mantissa as u128
        let abs = if self.is_sign_negative() {
            -*self
        } else {
            *self
        };

        let mantissa = abs.mantissa() as u128;

        // Convert to string representation for reliable base-10000 grouping
        let mantissa_str = mantissa.to_string();
        let total_len = mantissa_str.len();

        // Decimal point position from left: total_len - scale
        let point_pos = total_len.saturating_sub(scale as usize);

        // Pad left so integer part is multiple of 4
        let int_part_len = point_pos;
        let int_pad = (4 - (int_part_len % 4)) % 4;

        // Pad right so fractional part is multiple of 4
        let frac_part_len = total_len - point_pos
            + (scale as usize).saturating_sub(total_len);
        let frac_pad = (4 - (frac_part_len % 4)) % 4;

        let mut padded = String::new();

        if total_len <= scale as usize {
            // Pure fractional: construct full fractional digits string
            // e.g., 0.0000000001 → extra_zeros=9, mantissa="1" → "0000000001"
            let extra_zeros = (scale as usize).saturating_sub(total_len);
            for _ in 0..extra_zeros {
                padded.push('0');
            }
            padded.push_str(&mantissa_str);
            // Pad right to make multiple of 4
            let trail = (4 - (padded.len() % 4)) % 4;
            for _ in 0..trail {
                padded.push('0');
            }
        } else {
            // Has integer part
            for _ in 0..int_pad {
                padded.push('0');
            }
            padded.push_str(&mantissa_str);
            for _ in 0..frac_pad {
                padded.push('0');
            }
        }

        // Ensure total length is multiple of 4
        while padded.len() % 4 != 0 {
            padded.push('0');
        }

        let mut aligned_digits = Vec::new();
        for chunk in padded.as_bytes().chunks(4) {
            let s = std::str::from_utf8(chunk).unwrap_or("0000");
            aligned_digits.push(s.parse::<u16>().unwrap_or(0));
        }

        // Calculate weight: weight of the FIRST group in aligned_digits
        let weight: i16 = if total_len <= scale as usize {
            // Pure fractional: first group is the first 4 digits after decimal point
            -1
        } else {
            let groups_before_point = (int_part_len + int_pad) / 4;
            groups_before_point as i16 - 1
        };

        // Strip trailing zero groups (they're implied by dscale)
        while aligned_digits.last() == Some(&0) && aligned_digits.len() > 1 {
            aligned_digits.pop();
        }

        // Strip leading zero groups and adjust weight
        let mut final_weight = weight;
        while aligned_digits.first() == Some(&0) && aligned_digits.len() > 1 {
            aligned_digits.remove(0);
            final_weight -= 1;
        }

        let ndigits = aligned_digits.len() as i16;

        buf.put_i16(ndigits);
        buf.put_i16(final_weight);
        buf.put_u16(sign);
        buf.put_u16(scale);

        for d in &aligned_digits {
            buf.put_u16(*d);
        }

        Ok(())
    }
}

impl FromSql for Decimal {
    fn oid() -> Oid {
        Oid::NUMERIC
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() < 8 {
            return Err(Error::Decode(format!(
                "numeric: expected at least 8 bytes, got {}",
                buf.len()
            )));
        }

        let ndigits = i16::from_be_bytes([buf[0], buf[1]]);
        let weight = i16::from_be_bytes([buf[2], buf[3]]);
        let sign = u16::from_be_bytes([buf[4], buf[5]]);
        let dscale = u16::from_be_bytes([buf[6], buf[7]]);

        if sign == NUMERIC_NAN {
            return Err(Error::Decode(
                "numeric: NaN cannot be represented as Decimal".into(),
            ));
        }

        if ndigits == 0 {
            return Ok(Decimal::ZERO);
        }

        let expected_len = 8 + (ndigits as usize * 2);
        if buf.len() < expected_len {
            return Err(Error::Decode(format!(
                "numeric: expected {} bytes for {} digits, got {}",
                expected_len,
                ndigits,
                buf.len()
            )));
        }

        // Read base-10000 digits
        let mut digits = Vec::with_capacity(ndigits as usize);
        for i in 0..ndigits as usize {
            let d = u16::from_be_bytes([buf[8 + i * 2], buf[9 + i * 2]]);
            digits.push(d);
        }

        // Reconstruct the decimal value
        let mut result = Decimal::ZERO;
        let base = Decimal::new(10000, 0);

        for (i, &d) in digits.iter().enumerate() {
            let power = weight as i32 - i as i32;
            let digit_val = Decimal::new(d as i64, 0);

            if power >= 0 {
                let mut multiplier = Decimal::ONE;
                for _ in 0..power {
                    multiplier *= base;
                }
                result += digit_val * multiplier;
            } else {
                let mut divisor = Decimal::ONE;
                for _ in 0..(-power) {
                    divisor *= base;
                }
                result += digit_val / divisor;
            }
        }

        // Apply the correct scale
        result.rescale(dscale as u32);

        if sign == NUMERIC_NEG {
            result.set_sign_negative(true);
        }

        Ok(result)
    }
}
