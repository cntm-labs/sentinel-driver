use bytes::BytesMut;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use sentinel_driver::types::network::PgInet;
use sentinel_driver::types::{FromSql, Oid, ToSql};

fn roundtrip(val: &PgInet) {
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgInet::from_sql(&buf).ok();
    assert_eq!(decoded, Some(*val));
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
    val.to_sql(&mut buf).ok();
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
    val.to_sql(&mut buf).ok();
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
    val.to_sql(&mut buf).ok();
    let decoded = PgMacAddr::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
    assert_eq!(buf.len(), 6);
}

#[test]
fn test_cidr_roundtrip() {
    use sentinel_driver::types::network::PgCidr;
    let val = PgCidr {
        addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
        netmask: 8,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(buf[2], 1); // is_cidr = true
    let decoded = PgCidr::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_ipaddr_convenience_v4() {
    let val = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = IpAddr::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_ipaddr_convenience_v6() {
    let val = IpAddr::V6(Ipv6Addr::LOCALHOST);
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(buf.len(), 20);
    let decoded = IpAddr::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_cidr_ipv6_roundtrip() {
    use sentinel_driver::types::network::PgCidr;
    let val = PgCidr {
        addr: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0)),
        netmask: 32,
    };
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(buf[0], 3); // AF_INET6
    assert_eq!(buf[2], 1); // is_cidr = true
    let decoded = PgCidr::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_macaddr_wrong_size() {
    use sentinel_driver::types::network::PgMacAddr;
    assert!(PgMacAddr::from_sql(&[0u8; 5]).is_err());
    assert!(PgMacAddr::from_sql(&[0u8; 7]).is_err());
}

#[test]
fn test_inet_address_truncated() {
    // Valid header but truncated address data
    let buf = [2, 32, 0, 4, 192, 168]; // AF_INET, mask=32, not_cidr, len=4, but only 2 addr bytes
    assert!(PgInet::from_sql(&buf).is_err());
}

#[test]
fn test_inet_ipv4_wrong_addr_len() {
    // AF_INET but addr_len=16 (should be 4)
    let mut buf = vec![2, 32, 0, 16];
    buf.extend_from_slice(&[0u8; 16]);
    assert!(PgInet::from_sql(&buf).is_err());
}

#[test]
fn test_inet_ipv6_wrong_addr_len() {
    // AF_INET6 but addr_len=4 (should be 16)
    let buf = [3, 128, 0, 4, 0, 0, 0, 1];
    assert!(PgInet::from_sql(&buf).is_err());
}

#[test]
fn test_cidr_oid() {
    use sentinel_driver::types::network::PgCidr;
    let val = PgCidr {
        addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        netmask: 32,
    };
    assert_eq!(val.oid(), Oid::CIDR);
}

#[test]
fn test_macaddr_oid() {
    use sentinel_driver::types::network::PgMacAddr;
    assert_eq!(PgMacAddr([0; 6]).oid(), Oid::MACADDR);
}
