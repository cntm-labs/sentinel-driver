use bytes::{BufMut, BytesMut};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::error::{Error, Result};
use crate::types::{FromSql, Oid, ToSql};

const AF_INET: u8 = 2;
const AF_INET6: u8 = 3;

/// PostgreSQL INET type -- an IP address with optional subnet mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgInet {
    pub addr: IpAddr,
    pub netmask: u8,
}

/// PostgreSQL CIDR type -- a network address (same wire format as INET with is_cidr flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgCidr {
    pub addr: IpAddr,
    pub netmask: u8,
}

/// PostgreSQL MACADDR type -- 6-byte MAC address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PgMacAddr(pub [u8; 6]);

// -- INET / CIDR shared helpers --

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

// -- PgInet --

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

// -- PgCidr --

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

// -- PgMacAddr --

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

// -- std::net::IpAddr convenience impls --

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
