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
    // ToSql::oid (instance method)
    let val = PgCidr {
        addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        netmask: 32,
    };
    assert_eq!(val.oid(), Oid::CIDR);
    // FromSql::oid (static method) — covers line 138-140
    assert_eq!(<PgCidr as FromSql>::oid(), Oid::CIDR);
}

#[test]
fn test_macaddr_oid() {
    use sentinel_driver::types::network::PgMacAddr;
    // ToSql::oid (instance method)
    assert_eq!(PgMacAddr([0; 6]).oid(), Oid::MACADDR);
    // FromSql::oid (static method) — covers line 162-164
    assert_eq!(<PgMacAddr as FromSql>::oid(), Oid::MACADDR);
}

#[test]
fn test_inet_from_sql_oid() {
    // FromSql::oid (static method) — covers line 114-116
    assert_eq!(<PgInet as FromSql>::oid(), Oid::INET);
}

#[test]
fn test_ipaddr_oid() {
    // ToSql::oid (instance method) — covers line 177-179
    let v4 = IpAddr::V4(Ipv4Addr::LOCALHOST);
    assert_eq!(v4.oid(), Oid::INET);
    let v6 = IpAddr::V6(Ipv6Addr::LOCALHOST);
    assert_eq!(v6.oid(), Oid::INET);
    // FromSql::oid (static method) — covers line 192-194
    assert_eq!(<IpAddr as FromSql>::oid(), Oid::INET);
}

#[test]
fn test_inet_ipv6_address_truncated() {
    // AF_INET6, mask=128, not_cidr, addr_len=16, but only 8 bytes of addr data
    let mut buf = vec![3, 128, 0, 16];
    buf.extend_from_slice(&[0u8; 8]); // only 8 of 16 address bytes
    assert!(PgInet::from_sql(&buf).is_err());
}

#[test]
fn test_cidr_from_sql_ipv4() {
    use sentinel_driver::types::network::PgCidr;
    // Manually build a CIDR binary: AF_INET, mask=24, is_cidr=1, len=4, 10.0.0.0
    let buf = [2, 24, 1, 4, 10, 0, 0, 0];
    let decoded = PgCidr::from_sql(&buf).ok();
    assert_eq!(
        decoded,
        Some(PgCidr {
            addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
            netmask: 24,
        })
    );
}

#[test]
fn test_cidr_from_sql_ipv6() {
    use sentinel_driver::types::network::PgCidr;
    // Manually build an IPv6 CIDR binary
    let mut buf = vec![3, 64, 1, 16]; // AF_INET6, mask=64, is_cidr=1, len=16
    buf.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let decoded = PgCidr::from_sql(&buf).ok();
    assert!(decoded.is_some());
    let decoded = decoded.as_ref();
    assert_eq!(decoded.map(|c| c.netmask), Some(64));
}

#[test]
fn test_inet_from_sql_ipv4() {
    // Manually decode an IPv4 INET from raw bytes
    let buf = [2, 32, 0, 4, 127, 0, 0, 1]; // AF_INET, mask=32, not_cidr, len=4, 127.0.0.1
    let decoded = PgInet::from_sql(&buf).ok();
    assert_eq!(
        decoded,
        Some(PgInet {
            addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            netmask: 32,
        })
    );
}

#[test]
fn test_inet_from_sql_ipv6() {
    // Manually decode an IPv6 INET from raw bytes
    let mut buf = vec![3, 128, 0, 16]; // AF_INET6, mask=128, not_cidr, len=16
    buf.extend_from_slice(&Ipv6Addr::LOCALHOST.octets());
    let decoded = PgInet::from_sql(&buf).ok();
    assert_eq!(
        decoded,
        Some(PgInet {
            addr: IpAddr::V6(Ipv6Addr::LOCALHOST),
            netmask: 128,
        })
    );
}

#[test]
fn test_ipaddr_from_sql_v6() {
    // Manually decode IpAddr from raw IPv6 bytes
    let mut buf = vec![3, 128, 0, 16];
    buf.extend_from_slice(&Ipv6Addr::LOCALHOST.octets());
    let decoded = IpAddr::from_sql(&buf).ok();
    assert_eq!(decoded, Some(IpAddr::V6(Ipv6Addr::LOCALHOST)));
}

// -- Array roundtrip tests --

#[test]
fn test_inet_array_roundtrip() {
    let val = vec![
        PgInet {
            addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            netmask: 32,
        },
        PgInet {
            addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            netmask: 24,
        },
    ];
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Vec::<PgInet>::from_sql(&buf).ok();
    assert_eq!(decoded.as_ref(), Some(&val));
    assert_eq!(val.oid(), Oid::INET_ARRAY);
}

#[test]
fn test_cidr_array_roundtrip() {
    use sentinel_driver::types::network::PgCidr;
    let val = vec![PgCidr {
        addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
        netmask: 8,
    }];
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Vec::<PgCidr>::from_sql(&buf).ok();
    assert_eq!(decoded.as_ref(), Some(&val));
    assert_eq!(val.oid(), Oid::CIDR_ARRAY);
}

#[test]
fn test_macaddr_array_roundtrip() {
    use sentinel_driver::types::network::PgMacAddr;
    let val = vec![
        PgMacAddr([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03]),
        PgMacAddr([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
    ];
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Vec::<PgMacAddr>::from_sql(&buf).ok();
    assert_eq!(decoded.as_ref(), Some(&val));
    assert_eq!(val.oid(), Oid::MACADDR_ARRAY);
}

#[test]
fn test_inet_array_empty() {
    let val: Vec<PgInet> = vec![];
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = Vec::<PgInet>::from_sql(&buf).ok();
    assert_eq!(decoded, Some(vec![]));
}
