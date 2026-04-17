use bytes::BytesMut;
use sentinel_driver::types::ltree::{PgLQuery, PgLTree};
use sentinel_driver::types::{FromSql, Oid, ToSql};

// ── PgLTree ─────────────────────────────────────────

#[test]
fn test_ltree_roundtrip() {
    let val = PgLTree("top.science.astronomy".into());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLTree::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_ltree_single_label() {
    let val = PgLTree("root".into());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLTree::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_ltree_empty() {
    let val = PgLTree(String::new());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLTree::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_ltree_oid() {
    let val = PgLTree("a.b".into());
    assert_eq!(val.oid(), Oid::TEXT);
    assert_eq!(<PgLTree as FromSql>::oid(), Oid::TEXT);
}

#[test]
fn test_ltree_wire_format() {
    let val = PgLTree("a.b.c".into());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], b"a.b.c");
}

#[test]
fn test_ltree_display() {
    let val = PgLTree("top.science".into());
    assert_eq!(val.to_string(), "top.science");
}

#[test]
fn test_ltree_from_str() {
    let val: PgLTree = "top.science.astronomy".parse().unwrap();
    assert_eq!(val.0, "top.science.astronomy");
}

// ── PgLQuery ────────────────────────────────────────

#[test]
fn test_lquery_roundtrip() {
    let val = PgLQuery("*.science.*".into());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLQuery::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}

#[test]
fn test_lquery_oid() {
    let val = PgLQuery("*.a".into());
    assert_eq!(val.oid(), Oid::TEXT);
    assert_eq!(<PgLQuery as FromSql>::oid(), Oid::TEXT);
}

#[test]
fn test_lquery_wire_format() {
    let val = PgLQuery("top.*{1,3}.astronomy".into());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    assert_eq!(&buf[..], b"top.*{1,3}.astronomy");
}

#[test]
fn test_lquery_display() {
    let val = PgLQuery("*.science.*".into());
    assert_eq!(val.to_string(), "*.science.*");
}

#[test]
fn test_lquery_from_str() {
    let val: PgLQuery = "*.science.*".parse().unwrap();
    assert_eq!(val.0, "*.science.*");
}

// ── Unicode ─────────────────────────────────────────

#[test]
fn test_ltree_unicode() {
    let val = PgLTree("top.science.astronomy".into());
    let mut buf = BytesMut::new();
    val.to_sql(&mut buf).ok();
    let decoded = PgLTree::from_sql(&buf).ok();
    assert_eq!(decoded, Some(val));
}
