# Phase 1A: Core Type Expansion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add NUMERIC/Decimal, INET/CIDR, Interval, and Range type support to close the biggest type coverage gap against sqlx and tokio-postgres.

**Architecture:** Each type gets its own module under `types/` with `ToSql` + `FromSql` impls following PG binary wire format. Feature-gated types (`rust_decimal`, `ipnetwork`) use Cargo feature flags. Built-in types (interval, range) use custom structs with no external dependencies.

**Tech Stack:** Rust, bytes crate, rust_decimal (optional), ipnetwork (optional), chrono (existing)

---

## Task 1: PgInterval Type (Built-in, No Dependencies)

**Files:**
- Create: `crates/sentinel-driver/src/types/interval.rs`
- Modify: `crates/sentinel-driver/src/types/mod.rs` (add `pub mod interval;` + OIDs)
- Modify: `crates/sentinel-driver/src/types/builtin.rs` (register INTERVAL)
- Test: `tests/core/types_interval.rs`
- Modify: `tests/core/mod.rs` (add `mod types_interval;`)

**Step 1: Add OID constants**

In `crates/sentinel-driver/src/types/mod.rs`, add to `impl Oid`:

```rust
pub const INTERVAL: Oid = Oid(1186);
pub const INTERVAL_ARRAY: Oid = Oid(1187);
```

**Step 2: Register in builtin.rs**

In `crates/sentinel-driver/src/types/builtin.rs`, add to `BUILTIN_TYPES`:

```rust
TypeInfo {
    oid: Oid::INTERVAL,
    name: "interval",
    array_oid: Some(Oid::INTERVAL_ARRAY),
},
```

**Step 3: Write the failing tests**

Create `tests/core/types_interval.rs`:

```rust
use bytes::BytesMut;
use sentinel_driver::types::interval::PgInterval;
use sentinel_driver::types::{FromSql, ToSql};

fn roundtrip(val: &PgInterval) {
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded = PgInterval::from_sql(&buf).unwrap();
    assert_eq!(decoded, *val);
}

#[test]
fn test_interval_zero() {
    roundtrip(&PgInterval {
        months: 0,
        days: 0,
        microseconds: 0,
    });
}

#[test]
fn test_interval_one_month() {
    roundtrip(&PgInterval {
        months: 1,
        days: 0,
        microseconds: 0,
    });
}

#[test]
fn test_interval_complex() {
    // 1 year, 2 months, 3 days, 4 hours, 5 minutes, 6 seconds
    roundtrip(&PgInterval {
        months: 14,
        days: 3,
        microseconds: 14_706_000_000,
    });
}

#[test]
fn test_interval_negative() {
    roundtrip(&PgInterval {
        months: -6,
        days: -15,
        microseconds: -3_600_000_000,
    });
}

#[test]
fn test_interval_encode_wire_format() {
    let mut buf = BytesMut::new();
    let val = PgInterval {
        months: 2,
        days: 10,
        microseconds: 3_600_000_000, // 1 hour
    };
    val.to_sql(&mut buf).unwrap();
    // PG binary: microseconds(i64) + days(i32) + months(i32) = 16 bytes
    assert_eq!(buf.len(), 16);
    // microseconds in BE
    assert_eq!(
        &buf[0..8],
        &3_600_000_000i64.to_be_bytes()
    );
    // days in BE
    assert_eq!(&buf[8..12], &10i32.to_be_bytes());
    // months in BE
    assert_eq!(&buf[12..16], &2i32.to_be_bytes());
}

#[test]
fn test_interval_decode_too_short() {
    let buf = [0u8; 10];
    assert!(PgInterval::from_sql(&buf).is_err());
}

#[test]
fn test_interval_oid() {
    use sentinel_driver::types::Oid;
    let val = PgInterval {
        months: 0,
        days: 0,
        microseconds: 0,
    };
    assert_eq!(val.oid(), Oid::INTERVAL);
    assert_eq!(PgInterval::oid(), Oid::INTERVAL);
}
```

Add `mod types_interval;` to `tests/core/mod.rs`.

**Step 4: Run tests to verify they fail**

Run: `cargo test --lib --test core_tests types_interval -- --no-capture`
Expected: Compilation errors — `interval` module doesn't exist yet.

**Step 5: Write the implementation**

Create `crates/sentinel-driver/src/types/interval.rs`:

```rust
use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL INTERVAL type.
///
/// Stored as three components matching PG's internal representation:
/// - `months` — number of months (years × 12 + months)
/// - `days` — number of days (not normalized to months)
/// - `microseconds` — time component in microseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgInterval {
    pub months: i32,
    pub days: i32,
    pub microseconds: i64,
}

impl ToSql for PgInterval {
    fn oid(&self) -> Oid {
        Oid::INTERVAL
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i64(self.microseconds);
        buf.put_i32(self.days);
        buf.put_i32(self.months);
        Ok(())
    }
}

impl FromSql for PgInterval {
    fn oid() -> Oid {
        Oid::INTERVAL
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 16 {
            return Err(Error::Decode(format!(
                "interval: expected 16 bytes, got {}",
                buf.len()
            )));
        }

        let microseconds = i64::from_be_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]);
        let days = i32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let months = i32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);

        Ok(PgInterval {
            months,
            days,
            microseconds,
        })
    }
}
```

Add `pub mod interval;` to `crates/sentinel-driver/src/types/mod.rs`.

**Step 6: Run tests to verify they pass**

Run: `cargo test --lib --test core_tests types_interval -- --no-capture`
Expected: All 7 tests PASS.

**Step 7: Add array support**

Add to `crates/sentinel-driver/src/types/encode.rs`:

```rust
impl_array_to_sql!(crate::types::interval::PgInterval, Oid::INTERVAL_ARRAY, Oid::INTERVAL);
```

Add to `crates/sentinel-driver/src/types/decode.rs`:

```rust
impl_array_from_sql!(crate::types::interval::PgInterval, Oid::INTERVAL_ARRAY, Oid::INTERVAL);
```

**Step 8: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests PASS, no regressions.

**Step 9: Lint**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings.

**Step 10: Commit**

```bash
git add crates/sentinel-driver/src/types/interval.rs \
       crates/sentinel-driver/src/types/mod.rs \
       crates/sentinel-driver/src/types/builtin.rs \
       crates/sentinel-driver/src/types/encode.rs \
       crates/sentinel-driver/src/types/decode.rs \
       tests/core/types_interval.rs \
       tests/core/mod.rs
git commit -m "feat(types): add PgInterval type with binary encode/decode"
```

---

## Task 2: INET / CIDR Types (Built-in IpAddr + Optional ipnetwork)

**Files:**
- Create: `crates/sentinel-driver/src/types/network.rs`
- Modify: `crates/sentinel-driver/src/types/mod.rs` (add OIDs + module)
- Modify: `crates/sentinel-driver/src/types/builtin.rs` (register types)
- Modify: `crates/sentinel-driver/Cargo.toml` (add optional `ipnetwork`)
- Test: `tests/core/types_network.rs`
- Modify: `tests/core/mod.rs`

**Step 1: Add OID constants**

In `crates/sentinel-driver/src/types/mod.rs`, add to `impl Oid`:

```rust
pub const INET: Oid = Oid(869);
pub const CIDR: Oid = Oid(650);
pub const INET_ARRAY: Oid = Oid(1041);
pub const CIDR_ARRAY: Oid = Oid(651);
pub const MACADDR: Oid = Oid(829);
pub const MACADDR_ARRAY: Oid = Oid(1040);
```

**Step 2: Register in builtin.rs**

Add entries for INET, CIDR, MACADDR.

**Step 3: Add optional dependency**

In `crates/sentinel-driver/Cargo.toml`:

```toml
[dependencies]
ipnetwork = { version = "0.20", optional = true }

[features]
default = ["derive"]
derive = ["dep:sentinel-derive"]
with-ipnetwork = ["dep:ipnetwork"]
```

**Step 4: Write the failing tests**

Create `tests/core/types_network.rs`:

```rust
use bytes::BytesMut;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use sentinel_driver::types::network::PgInet;
use sentinel_driver::types::{FromSql, Oid, ToSql};

fn roundtrip(val: &PgInet) {
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded = PgInet::from_sql(&buf).unwrap();
    assert_eq!(decoded, *val);
}

#[test]
fn test_inet_ipv4_host() {
    roundtrip(&PgInet {
        addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        netmask: 32,
    });
}

#[test]
fn test_inet_ipv4_subnet() {
    roundtrip(&PgInet {
        addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
        netmask: 8,
    });
}

#[test]
fn test_inet_ipv6_host() {
    roundtrip(&PgInet {
        addr: IpAddr::V6(Ipv6Addr::LOCALHOST),
        netmask: 128,
    });
}

#[test]
fn test_inet_ipv6_subnet() {
    roundtrip(&PgInet {
        addr: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0)),
        netmask: 32,
    });
}

#[test]
fn test_inet_encode_wire_format_ipv4() {
    let mut buf = BytesMut::new();
    let val = PgInet {
        addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        netmask: 24,
    };
    val.to_sql(&mut buf).unwrap();
    // family(1) + netmask(1) + is_cidr(1) + length(1) + addr(4) = 8 bytes
    assert_eq!(buf.len(), 8);
    assert_eq!(buf[0], 2); // AF_INET
    assert_eq!(buf[1], 24); // netmask
    assert_eq!(buf[2], 0); // is_cidr = false (INET)
    assert_eq!(buf[3], 4); // address length
    assert_eq!(&buf[4..8], &[192, 168, 1, 1]);
}

#[test]
fn test_inet_encode_wire_format_ipv6() {
    let mut buf = BytesMut::new();
    let val = PgInet {
        addr: IpAddr::V6(Ipv6Addr::LOCALHOST),
        netmask: 128,
    };
    val.to_sql(&mut buf).unwrap();
    // family(1) + netmask(1) + is_cidr(1) + length(1) + addr(16) = 20 bytes
    assert_eq!(buf.len(), 20);
    assert_eq!(buf[0], 3); // AF_INET6
    assert_eq!(buf[3], 16); // address length
}

#[test]
fn test_inet_oid() {
    let val = PgInet {
        addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        netmask: 32,
    };
    assert_eq!(val.oid(), Oid::INET);
}

#[test]
fn test_inet_decode_too_short() {
    let buf = [0u8; 2];
    assert!(PgInet::from_sql(&buf).is_err());
}

#[test]
fn test_inet_decode_invalid_family() {
    let buf = [99, 32, 0, 4, 1, 2, 3, 4]; // family=99 invalid
    assert!(PgInet::from_sql(&buf).is_err());
}

#[test]
fn test_macaddr_roundtrip() {
    use sentinel_driver::types::network::PgMacAddr;
    let val = PgMacAddr([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03]);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).unwrap();
    let decoded = PgMacAddr::from_sql(&buf).unwrap();
    assert_eq!(decoded, val);
    assert_eq!(buf.len(), 6);
}
```

Add `mod types_network;` to `tests/core/mod.rs`.

**Step 5: Write the implementation**

Create `crates/sentinel-driver/src/types/network.rs`:

```rust
use bytes::{BufMut, BytesMut};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

const AF_INET: u8 = 2;
const AF_INET6: u8 = 3;

/// PostgreSQL INET type — an IP address with optional subnet mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgInet {
    pub addr: IpAddr,
    pub netmask: u8,
}

/// PostgreSQL CIDR type — a network address (same wire format as INET with is_cidr flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgCidr {
    pub addr: IpAddr,
    pub netmask: u8,
}

/// PostgreSQL MACADDR type — 6-byte MAC address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgMacAddr(pub [u8; 6]);

// ── INET ────────────────────────────────────────────

fn encode_inet(addr: &IpAddr, netmask: u8, is_cidr: bool, buf: &mut BytesMut) {
    match addr {
        IpAddr::V4(v4) => {
            buf.put_u8(AF_INET);
            buf.put_u8(netmask);
            buf.put_u8(u8::from(is_cidr));
            buf.put_u8(4);
            buf.put_slice(&v4.octets());
        }
        IpAddr::V6(v6) => {
            buf.put_u8(AF_INET6);
            buf.put_u8(netmask);
            buf.put_u8(u8::from(is_cidr));
            buf.put_u8(16);
            buf.put_slice(&v6.octets());
        }
    }
}

fn decode_inet(buf: &[u8]) -> Result<(IpAddr, u8, bool)> {
    if buf.len() < 4 {
        return Err(Error::Decode(format!(
            "inet: expected at least 4 bytes, got {}",
            buf.len()
        )));
    }

    let family = buf[0];
    let netmask = buf[1];
    let is_cidr = buf[2] != 0;
    let addr_len = buf[3] as usize;

    if buf.len() < 4 + addr_len {
        return Err(Error::Decode(format!(
            "inet: address truncated, expected {} bytes, got {}",
            4 + addr_len,
            buf.len()
        )));
    }

    let addr = match family {
        AF_INET => {
            if addr_len != 4 {
                return Err(Error::Decode(format!(
                    "inet: IPv4 address should be 4 bytes, got {addr_len}"
                )));
            }
            IpAddr::V4(Ipv4Addr::new(buf[4], buf[5], buf[6], buf[7]))
        }
        AF_INET6 => {
            if addr_len != 16 {
                return Err(Error::Decode(format!(
                    "inet: IPv6 address should be 16 bytes, got {addr_len}"
                )));
            }
            let octets: [u8; 16] = buf[4..20]
                .try_into()
                .map_err(|_| Error::Decode("inet: IPv6 slice error".into()))?;
            IpAddr::V6(Ipv6Addr::from(octets))
        }
        _ => {
            return Err(Error::Decode(format!(
                "inet: unknown address family {family}"
            )));
        }
    };

    Ok((addr, netmask, is_cidr))
}

impl ToSql for PgInet {
    fn oid(&self) -> Oid {
        Oid::INET
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        encode_inet(&self.addr, self.netmask, false, buf);
        Ok(())
    }
}

impl FromSql for PgInet {
    fn oid() -> Oid {
        Oid::INET
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let (addr, netmask, _is_cidr) = decode_inet(buf)?;
        Ok(PgInet { addr, netmask })
    }
}

impl ToSql for PgCidr {
    fn oid(&self) -> Oid {
        Oid::CIDR
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        encode_inet(&self.addr, self.netmask, true, buf);
        Ok(())
    }
}

impl FromSql for PgCidr {
    fn oid() -> Oid {
        Oid::CIDR
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let (addr, netmask, _is_cidr) = decode_inet(buf)?;
        Ok(PgCidr { addr, netmask })
    }
}

// ── MACADDR ─────────────────────────────────────────

impl ToSql for PgMacAddr {
    fn oid(&self) -> Oid {
        Oid::MACADDR
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_slice(&self.0);
        Ok(())
    }
}

impl FromSql for PgMacAddr {
    fn oid() -> Oid {
        Oid::MACADDR
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 6] = buf
            .try_into()
            .map_err(|_| Error::Decode(format!("macaddr: expected 6 bytes, got {}", buf.len())))?;
        Ok(PgMacAddr(arr))
    }
}

// ── std::net::IpAddr convenience impls ──────────────

impl ToSql for IpAddr {
    fn oid(&self) -> Oid {
        Oid::INET
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        let netmask = match self {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        encode_inet(self, netmask, false, buf);
        Ok(())
    }
}

impl FromSql for IpAddr {
    fn oid() -> Oid {
        Oid::INET
    }

    fn from_sql(buf: &[u8]) -> Result<Self> {
        let (addr, _netmask, _is_cidr) = decode_inet(buf)?;
        Ok(addr)
    }
}
```

Add `pub mod network;` to `types/mod.rs`.

**Step 6: Run tests**

Run: `cargo test --workspace`
Expected: All tests PASS.

**Step 7: Lint + Commit**

```bash
cargo clippy --workspace -- -D warnings
git add -A
git commit -m "feat(types): add INET, CIDR, MACADDR with PgInet/PgCidr/PgMacAddr"
```

---

## Task 3: NUMERIC / Decimal (Feature-gated: rust_decimal)

**Files:**
- Create: `crates/sentinel-driver/src/types/numeric.rs`
- Modify: `crates/sentinel-driver/src/types/mod.rs` (add OIDs + conditional module)
- Modify: `crates/sentinel-driver/src/types/builtin.rs`
- Modify: `crates/sentinel-driver/Cargo.toml`
- Modify: `Cargo.toml` (workspace root — ensure rust_decimal resolves)
- Test: `tests/core/types_numeric.rs`
- Modify: `tests/core/mod.rs`

**Step 1: Add OID constants**

```rust
pub const NUMERIC: Oid = Oid(1700);
pub const NUMERIC_ARRAY: Oid = Oid(1231);
```

**Step 2: Add dependency**

In `crates/sentinel-driver/Cargo.toml`:

```toml
[dependencies]
rust_decimal = { version = "1", optional = true }

[features]
with-rust-decimal = ["dep:rust_decimal"]
```

**Step 3: Write the failing tests**

Create `tests/core/types_numeric.rs`:

```rust
#[cfg(feature = "with-rust-decimal")]
mod numeric_tests {
    use bytes::BytesMut;
    use rust_decimal::Decimal;
    use sentinel_driver::types::{FromSql, Oid, ToSql};

    fn roundtrip(val: &Decimal) {
        let mut buf = BytesMut::new();
        val.to_sql(&mut buf).unwrap();
        let decoded = Decimal::from_sql(&buf).unwrap();
        assert_eq!(decoded, *val);
    }

    #[test]
    fn test_numeric_zero() {
        roundtrip(&Decimal::ZERO);
    }

    #[test]
    fn test_numeric_positive_integer() {
        roundtrip(&Decimal::new(12345, 0));
    }

    #[test]
    fn test_numeric_negative() {
        roundtrip(&Decimal::new(-99999, 0));
    }

    #[test]
    fn test_numeric_with_scale() {
        roundtrip(&Decimal::new(31415, 4)); // 3.1415
    }

    #[test]
    fn test_numeric_small_decimal() {
        roundtrip(&Decimal::new(1, 10)); // 0.0000000001
    }

    #[test]
    fn test_numeric_large() {
        roundtrip(&Decimal::new(999_999_999_999, 2)); // 9999999999.99
    }

    #[test]
    fn test_numeric_one() {
        roundtrip(&Decimal::ONE);
    }

    #[test]
    fn test_numeric_oid() {
        let val = Decimal::ZERO;
        assert_eq!(val.oid(), Oid::NUMERIC);
    }
}
```

**Step 4: Write the implementation**

Create `crates/sentinel-driver/src/types/numeric.rs`:

```rust
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
const BASE: u32 = 10_000;

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

        // Convert mantissa to base-10000 digits
        let mut base10k_digits = Vec::new();
        let mut remaining = mantissa;
        if remaining == 0 {
            base10k_digits.push(0u16);
        } else {
            while remaining > 0 {
                base10k_digits.push((remaining % BASE as u128) as u16);
                remaining /= BASE as u128;
            }
            base10k_digits.reverse();
        }

        // Calculate weight: the total number of decimal digits in mantissa
        // determines position of first base-10000 group
        let total_decimal_digits = decimal_digit_count(mantissa);
        let integer_digits = if total_decimal_digits > scale as u32 {
            total_decimal_digits - scale as u32
        } else {
            0
        };

        // Weight = number of base-10000 groups before decimal point - 1
        let groups_before_point = (integer_digits + 3) / 4;
        let weight = if integer_digits == 0 {
            // Pure fractional: weight = -(leading_zero_groups + 1)
            let leading_zeros = scale as u32 - total_decimal_digits;
            let skip_groups = leading_zeros / 4;
            -(skip_groups as i16 + 1)
        } else {
            groups_before_point as i16 - 1
        };

        // We need to pad the digits to align with base-10000 groups
        // Re-derive digits aligned to the weight
        let mut aligned_digits = Vec::new();
        let start_power = (weight as i32 + 1) * 4; // decimal position of first group
        let end_power = start_power - (base10k_digits.len() as i32 * 4);

        // Re-extract from mantissa with proper alignment
        let shift_amount = if scale as i32 > total_decimal_digits as i32 - (end_power) {
            0i32
        } else {
            0i32
        };
        let _ = shift_amount;

        // Simpler approach: convert to string, pad, chunk into groups of 4
        let mantissa_str = mantissa.to_string();
        let total_len = mantissa_str.len();

        // Decimal point position from left: total_len - scale
        let point_pos = if total_len > scale as usize {
            total_len - scale as usize
        } else {
            0
        };

        // Pad left so integer part is multiple of 4
        let int_part_len = point_pos;
        let int_pad = (4 - (int_part_len % 4)) % 4;

        // Pad right so fractional part is multiple of 4
        let frac_part_len = total_len - point_pos + (if total_len < scale as usize { scale as usize - total_len } else { 0 });
        let frac_pad = (4 - (frac_part_len % 4)) % 4;

        let mut padded = String::new();
        // Leading zeros for integer alignment
        for _ in 0..int_pad {
            padded.push('0');
        }
        // Leading zeros if number is like 0.000123 (total_len < scale)
        if total_len < scale as usize {
            let extra_zeros = scale as usize - total_len;
            // Align the fractional leading zeros to 4-digit groups
            let total_frac_zeros = int_pad + extra_zeros;
            padded.clear();
            for _ in 0..((total_frac_zeros + 3) / 4 * 4) {
                padded.push('0');
            }
        }
        padded.push_str(&mantissa_str);
        // Trailing zeros for fractional alignment
        for _ in 0..frac_pad {
            padded.push('0');
        }

        // Ensure total length is multiple of 4
        while padded.len() % 4 != 0 {
            padded.push('0');
        }

        aligned_digits.clear();
        for chunk in padded.as_bytes().chunks(4) {
            let s = std::str::from_utf8(chunk).unwrap_or("0000");
            aligned_digits.push(s.parse::<u16>().unwrap_or(0));
        }

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

fn decimal_digit_count(mut n: u128) -> u32 {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    while n > 0 {
        n /= 10;
        count += 1;
    }
    count
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
            return Err(Error::Decode("numeric: NaN cannot be represented as Decimal".into()));
        }

        if ndigits == 0 {
            return Ok(Decimal::ZERO);
        }

        let expected_len = 8 + (ndigits as usize * 2);
        if buf.len() < expected_len {
            return Err(Error::Decode(format!(
                "numeric: expected {} bytes for {} digits, got {}",
                expected_len, ndigits, buf.len()
            )));
        }

        // Read base-10000 digits
        let mut digits = Vec::with_capacity(ndigits as usize);
        for i in 0..ndigits as usize {
            let d = u16::from_be_bytes([buf[8 + i * 2], buf[9 + i * 2]]);
            digits.push(d);
        }

        // Reconstruct the decimal value
        // weight means: first digit represents value * 10000^weight
        // dscale means: number of decimal places after point
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
```

Add to `types/mod.rs`:

```rust
#[cfg(feature = "with-rust-decimal")]
pub mod numeric;
```

**Step 5: Run tests**

Run: `cargo test --workspace --features with-rust-decimal`
Expected: All tests PASS.

**Step 6: Lint + Commit**

```bash
cargo clippy --workspace --features with-rust-decimal -- -D warnings
git add -A
git commit -m "feat(types): add NUMERIC/Decimal support (feature: with-rust-decimal)"
```

---

## Task 4: Range Types (Built-in PgRange<T>)

**Files:**
- Create: `crates/sentinel-driver/src/types/range.rs`
- Modify: `crates/sentinel-driver/src/types/mod.rs` (add OIDs + module)
- Modify: `crates/sentinel-driver/src/types/builtin.rs`
- Test: `tests/core/types_range.rs`
- Modify: `tests/core/mod.rs`

**Step 1: Add OID constants**

```rust
pub const INT4RANGE: Oid = Oid(3904);
pub const INT8RANGE: Oid = Oid(3926);
pub const NUMRANGE: Oid = Oid(3906);
pub const TSRANGE: Oid = Oid(3908);
pub const TSTZRANGE: Oid = Oid(3910);
pub const DATERANGE: Oid = Oid(3912);
pub const INT4RANGE_ARRAY: Oid = Oid(3905);
pub const INT8RANGE_ARRAY: Oid = Oid(3927);
pub const NUMRANGE_ARRAY: Oid = Oid(3907);
pub const TSRANGE_ARRAY: Oid = Oid(3909);
pub const TSTZRANGE_ARRAY: Oid = Oid(3911);
pub const DATERANGE_ARRAY: Oid = Oid(3913);
```

**Step 2: Write the failing tests**

Create `tests/core/types_range.rs`:

```rust
use bytes::BytesMut;
use sentinel_driver::types::range::{PgRange, RangeBound};
use sentinel_driver::types::{FromSql, Oid, ToSql};

#[test]
fn test_range_empty_i32() {
    let range: PgRange<i32> = PgRange::empty(Oid::INT4RANGE, Oid::INT4);
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).unwrap();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).unwrap();
    assert!(decoded.is_empty);
}

#[test]
fn test_range_inclusive_i32() {
    let range = PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Inclusive(10i32),
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).unwrap();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).unwrap();
    assert_eq!(decoded.lower, RangeBound::Inclusive(1));
    assert_eq!(decoded.upper, RangeBound::Inclusive(10));
}

#[test]
fn test_range_exclusive_i64() {
    let range = PgRange {
        lower: RangeBound::Exclusive(0i64),
        upper: RangeBound::Exclusive(100i64),
        is_empty: false,
        range_oid: Oid::INT8RANGE,
        element_oid: Oid::INT8,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).unwrap();
    let decoded = PgRange::<i64>::from_sql_with_oids(&buf, Oid::INT8RANGE, Oid::INT8).unwrap();
    assert_eq!(decoded.lower, RangeBound::Exclusive(0));
    assert_eq!(decoded.upper, RangeBound::Exclusive(100));
}

#[test]
fn test_range_unbounded_lower() {
    let range = PgRange {
        lower: RangeBound::<i32>::Unbounded,
        upper: RangeBound::Inclusive(50),
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).unwrap();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).unwrap();
    assert_eq!(decoded.lower, RangeBound::Unbounded);
    assert_eq!(decoded.upper, RangeBound::Inclusive(50));
}

#[test]
fn test_range_unbounded_both() {
    let range = PgRange {
        lower: RangeBound::<i32>::Unbounded,
        upper: RangeBound::<i32>::Unbounded,
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).unwrap();
    let decoded = PgRange::<i32>::from_sql_with_oids(&buf, Oid::INT4RANGE, Oid::INT4).unwrap();
    assert_eq!(decoded.lower, RangeBound::Unbounded);
    assert_eq!(decoded.upper, RangeBound::Unbounded);
}

#[test]
fn test_range_wire_format_empty() {
    let range: PgRange<i32> = PgRange::empty(Oid::INT4RANGE, Oid::INT4);
    let mut buf = BytesMut::new();
    range.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 1);
    assert_eq!(buf[0], 0x01); // RANGE_EMPTY flag
}

#[test]
fn test_range_oid() {
    let range = PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Inclusive(10i32),
        is_empty: false,
        range_oid: Oid::INT4RANGE,
        element_oid: Oid::INT4,
    };
    assert_eq!(range.oid(), Oid::INT4RANGE);
}
```

**Step 3: Write the implementation**

Create `crates/sentinel-driver/src/types/range.rs`:

```rust
use bytes::{BufMut, BytesMut};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

const RANGE_EMPTY: u8 = 0x01;
const RANGE_LB_INC: u8 = 0x02;
const RANGE_UB_INC: u8 = 0x04;
const RANGE_LB_INF: u8 = 0x08;
const RANGE_UB_INF: u8 = 0x10;

/// A bound of a PostgreSQL range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeBound<T> {
    Inclusive(T),
    Exclusive(T),
    Unbounded,
}

/// PostgreSQL range type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgRange<T> {
    pub lower: RangeBound<T>,
    pub upper: RangeBound<T>,
    pub is_empty: bool,
    pub range_oid: Oid,
    pub element_oid: Oid,
}

impl<T> PgRange<T> {
    pub fn empty(range_oid: Oid, element_oid: Oid) -> Self {
        PgRange {
            lower: RangeBound::Unbounded,
            upper: RangeBound::Unbounded,
            is_empty: true,
            range_oid,
            element_oid,
        }
    }
}

impl<T: ToSql> ToSql for PgRange<T> {
    fn oid(&self) -> Oid {
        self.range_oid
    }

    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        if self.is_empty {
            buf.put_u8(RANGE_EMPTY);
            return Ok(());
        }

        let mut flags: u8 = 0;

        match &self.lower {
            RangeBound::Inclusive(_) => flags |= RANGE_LB_INC,
            RangeBound::Exclusive(_) => {}
            RangeBound::Unbounded => flags |= RANGE_LB_INF,
        }

        match &self.upper {
            RangeBound::Inclusive(_) => flags |= RANGE_UB_INC,
            RangeBound::Exclusive(_) => {}
            RangeBound::Unbounded => flags |= RANGE_UB_INF,
        }

        buf.put_u8(flags);

        // Encode lower bound
        match &self.lower {
            RangeBound::Inclusive(v) | RangeBound::Exclusive(v) => {
                let len_pos = buf.len();
                buf.put_i32(0); // placeholder
                let data_start = buf.len();
                v.to_sql(buf)?;
                let data_len = (buf.len() - data_start) as i32;
                buf[len_pos..len_pos + 4].copy_from_slice(&data_len.to_be_bytes());
            }
            RangeBound::Unbounded => {}
        }

        // Encode upper bound
        match &self.upper {
            RangeBound::Inclusive(v) | RangeBound::Exclusive(v) => {
                let len_pos = buf.len();
                buf.put_i32(0); // placeholder
                let data_start = buf.len();
                v.to_sql(buf)?;
                let data_len = (buf.len() - data_start) as i32;
                buf[len_pos..len_pos + 4].copy_from_slice(&data_len.to_be_bytes());
            }
            RangeBound::Unbounded => {}
        }

        Ok(())
    }
}

impl<T: FromSql> PgRange<T> {
    pub fn from_sql_with_oids(buf: &[u8], range_oid: Oid, element_oid: Oid) -> Result<Self> {
        if buf.is_empty() {
            return Err(Error::Decode("range: empty buffer".into()));
        }

        let flags = buf[0];

        if flags & RANGE_EMPTY != 0 {
            return Ok(PgRange::empty(range_oid, element_oid));
        }

        let mut offset = 1;

        let lower = if flags & RANGE_LB_INF != 0 {
            RangeBound::Unbounded
        } else {
            if offset + 4 > buf.len() {
                return Err(Error::Decode("range: lower bound truncated".into()));
            }
            let len = i32::from_be_bytes([
                buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3],
            ]) as usize;
            offset += 4;
            if offset + len > buf.len() {
                return Err(Error::Decode("range: lower bound data truncated".into()));
            }
            let val = T::from_sql(&buf[offset..offset + len])?;
            offset += len;
            if flags & RANGE_LB_INC != 0 {
                RangeBound::Inclusive(val)
            } else {
                RangeBound::Exclusive(val)
            }
        };

        let upper = if flags & RANGE_UB_INF != 0 {
            RangeBound::Unbounded
        } else {
            if offset + 4 > buf.len() {
                return Err(Error::Decode("range: upper bound truncated".into()));
            }
            let len = i32::from_be_bytes([
                buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3],
            ]) as usize;
            offset += 4;
            if offset + len > buf.len() {
                return Err(Error::Decode("range: upper bound data truncated".into()));
            }
            let val = T::from_sql(&buf[offset..offset + len])?;
            if flags & RANGE_UB_INC != 0 {
                RangeBound::Inclusive(val)
            } else {
                RangeBound::Exclusive(val)
            }
        };

        Ok(PgRange {
            lower,
            upper,
            is_empty: false,
            range_oid,
            element_oid,
        })
    }
}
```

Add `pub mod range;` to `types/mod.rs`.

**Step 4: Run tests**

Run: `cargo test --workspace`
Expected: All tests PASS.

**Step 5: Lint + Commit**

```bash
cargo clippy --workspace -- -D warnings
git add -A
git commit -m "feat(types): add PgRange<T> with support for int4range, int8range, tsrange, etc."
```

---

## Task 5: Remaining Tier B Types (Money, Geometric, XML, PG_LSN)

**Files:**
- Create: `crates/sentinel-driver/src/types/money.rs`
- Create: `crates/sentinel-driver/src/types/geometric.rs`
- Create: `crates/sentinel-driver/src/types/xml.rs`
- Create: `crates/sentinel-driver/src/types/lsn.rs`
- Modify: `crates/sentinel-driver/src/types/mod.rs`
- Modify: `crates/sentinel-driver/src/types/builtin.rs`
- Test: `tests/core/types_money.rs`
- Test: `tests/core/types_geometric.rs`
- Modify: `tests/core/mod.rs`

**Step 1: Add all remaining OID constants**

```rust
pub const MONEY: Oid = Oid(790);
pub const MONEY_ARRAY: Oid = Oid(791);
pub const POINT: Oid = Oid(600);
pub const LINE: Oid = Oid(628);
pub const LSEG: Oid = Oid(601);
pub const BOX: Oid = Oid(603);
pub const PATH: Oid = Oid(602);
pub const POLYGON: Oid = Oid(604);
pub const CIRCLE: Oid = Oid(718);
pub const XML: Oid = Oid(142);
pub const PG_LSN: Oid = Oid(3220);
```

**Step 2: Implement PgMoney**

`money.rs` — simple i64 wrapper:

```rust
use bytes::{BufMut, BytesMut};
use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

/// PostgreSQL MONEY type — stored as i64 cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgMoney(pub i64);

impl ToSql for PgMoney {
    fn oid(&self) -> Oid { Oid::MONEY }
    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_i64(self.0);
        Ok(())
    }
}

impl FromSql for PgMoney {
    fn oid() -> Oid { Oid::MONEY }
    fn from_sql(buf: &[u8]) -> Result<Self> {
        let arr: [u8; 8] = buf.try_into()
            .map_err(|_| Error::Decode(format!("money: expected 8 bytes, got {}", buf.len())))?;
        Ok(PgMoney(i64::from_be_bytes(arr)))
    }
}
```

**Step 3: Implement geometric types**

`geometric.rs` — PgPoint, PgLine, PgLSeg, PgBox, PgCircle (all f64-based):

```rust
use bytes::{BufMut, BytesMut};
use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgPoint { pub x: f64, pub y: f64 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgLine { pub a: f64, pub b: f64, pub c: f64 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgLSeg { pub start: PgPoint, pub end: PgPoint }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgBox { pub upper_right: PgPoint, pub lower_left: PgPoint }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PgCircle { pub center: PgPoint, pub radius: f64 }

// ToSql / FromSql for each type:
// PgPoint: 16 bytes (2 × f64)
// PgLine: 24 bytes (3 × f64)
// PgLSeg: 32 bytes (4 × f64)
// PgBox: 32 bytes (4 × f64)
// PgCircle: 24 bytes (2 × f64 + f64)

impl ToSql for PgPoint {
    fn oid(&self) -> Oid { Oid::POINT }
    fn to_sql(&self, buf: &mut BytesMut) -> Result<()> {
        buf.put_f64(self.x);
        buf.put_f64(self.y);
        Ok(())
    }
}

impl FromSql for PgPoint {
    fn oid() -> Oid { Oid::POINT }
    fn from_sql(buf: &[u8]) -> Result<Self> {
        if buf.len() != 16 {
            return Err(Error::Decode(format!("point: expected 16 bytes, got {}", buf.len())));
        }
        let x = f64::from_be_bytes(buf[0..8].try_into().unwrap());
        let y = f64::from_be_bytes(buf[8..16].try_into().unwrap());
        Ok(PgPoint { x, y })
    }
}

// Implement similarly for PgLine(24), PgLSeg(32), PgBox(32), PgCircle(24)
```

**Step 4: Implement XML (String wrapper) and PG_LSN (u64 wrapper)**

`xml.rs` — text encode/decode with XML OID.
`lsn.rs` — PgLsn(u64) with i64 encode (PG stores as uint64 but sends as i64 on wire).

**Step 5: Write tests for each type, run, verify, commit**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
git add -A
git commit -m "feat(types): add Money, Geometric, XML, PG_LSN types"
```

---

## Task 6: Update Feature Flags and Documentation

**Files:**
- Modify: `crates/sentinel-driver/Cargo.toml` (final feature flags)
- Modify: `crates/sentinel-driver/src/lib.rs` (re-export new types)
- Modify: `CLAUDE.md` (update type count)

**Step 1: Final Cargo.toml features**

```toml
[features]
default = ["derive"]
derive = ["dep:sentinel-derive"]
with-rust-decimal = ["dep:rust_decimal"]
with-ipnetwork = ["dep:ipnetwork"]
```

**Step 2: Re-export types from lib.rs**

Ensure `PgInterval`, `PgInet`, `PgCidr`, `PgMacAddr`, `PgRange`, `RangeBound`, `PgMoney`, `PgPoint`, `PgLSeg`, `PgBox`, `PgCircle`, `PgLine`, `PgLsn` are accessible via `sentinel_driver::types::*`.

**Step 3: Run full test suite with all features**

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(types): complete Phase 1A type expansion — 40+ OIDs supported"
```

---

## Summary

| Task | Type | OIDs Added | Effort |
|------|------|-----------|--------|
| 1 | PgInterval | 2 | Small |
| 2 | INET/CIDR/MACADDR | 6 | Medium |
| 3 | NUMERIC/Decimal | 2 | Large (complex encoding) |
| 4 | Range<T> | 12 | Medium |
| 5 | Money/Geometric/XML/LSN | ~12 | Medium |
| 6 | Feature flags + docs | 0 | Small |

**Total new OIDs:** ~34 (from 27 → 61)
**Dependencies added:** `rust_decimal` (optional), `ipnetwork` (optional)
