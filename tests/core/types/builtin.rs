use sentinel_driver::types::builtin::{lookup, lookup_by_name};
use sentinel_driver::types::Oid;

#[test]
fn test_lookup_by_oid() {
    let info = lookup(Oid::INT4).unwrap();
    assert_eq!(info.name, "int4");
    assert_eq!(info.array_oid, Some(Oid::INT4_ARRAY));
}

#[test]
fn test_lookup_by_name() {
    let info = lookup_by_name("uuid").unwrap();
    assert_eq!(info.oid, Oid::UUID);
}

#[test]
fn test_lookup_unknown() {
    assert!(lookup(Oid(99999)).is_none());
    assert!(lookup_by_name("nonexistent").is_none());
}

#[test]
fn test_oid_from_u32() {
    let oid = Oid::from(23u32);
    assert_eq!(oid, Oid::INT4);
}

#[test]
fn test_u32_from_oid() {
    let val: u32 = Oid::INT4.into();
    assert_eq!(val, 23);
}
