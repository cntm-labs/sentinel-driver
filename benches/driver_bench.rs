use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bytes::BytesMut;
use sentinel_driver::types::{FromSql, Oid, ToSql};

fn bench_i32_encode(c: &mut Criterion) {
    c.bench_function("i32_encode", |b| {
        b.iter(|| {
            let mut buf = BytesMut::with_capacity(4);
            black_box(42i32).to_sql(&mut buf).ok();
            black_box(buf);
        });
    });
}

fn bench_i32_decode(c: &mut Criterion) {
    let data = 42i32.to_be_bytes();
    c.bench_function("i32_decode", |b| {
        b.iter(|| {
            let val: i32 = FromSql::from_sql(black_box(&data)).expect("decode");
            black_box(val);
        });
    });
}

fn bench_string_encode(c: &mut Criterion) {
    let s = "hello world".to_string();
    c.bench_function("string_encode", |b| {
        b.iter(|| {
            let mut buf = BytesMut::with_capacity(32);
            black_box(&s).to_sql(&mut buf).ok();
            black_box(buf);
        });
    });
}

fn bench_uuid_encode(c: &mut Criterion) {
    let id = uuid::Uuid::new_v4();
    c.bench_function("uuid_encode", |b| {
        b.iter(|| {
            let mut buf = BytesMut::with_capacity(16);
            black_box(&id).to_sql(&mut buf).ok();
            black_box(buf);
        });
    });
}

fn bench_uuid_decode(c: &mut Criterion) {
    let id = uuid::Uuid::new_v4();
    let mut buf = BytesMut::new();
    id.to_sql(&mut buf).ok();
    let bytes = buf.freeze();
    c.bench_function("uuid_decode", |b| {
        b.iter(|| {
            let val: uuid::Uuid = FromSql::from_sql(black_box(&bytes)).expect("decode");
            black_box(val);
        });
    });
}

fn bench_oid_lookup(c: &mut Criterion) {
    c.bench_function("oid_lookup", |b| {
        b.iter(|| {
            let oid = black_box(Oid::INT4);
            black_box(sentinel_driver::types::builtin::lookup(oid));
        });
    });
}

fn bench_hstore_encode(c: &mut Criterion) {
    let mut map = std::collections::HashMap::new();
    for i in 0..10 {
        map.insert(format!("key_{i}"), Some(format!("value_{i}")));
    }
    c.bench_function("hstore_encode_10pairs", |b| {
        b.iter(|| {
            let mut buf = BytesMut::with_capacity(256);
            black_box(&map).to_sql(&mut buf).ok();
            black_box(buf);
        });
    });
}

fn bench_hstore_decode(c: &mut Criterion) {
    let mut map = std::collections::HashMap::new();
    for i in 0..10 {
        map.insert(format!("key_{i}"), Some(format!("value_{i}")));
    }
    let mut buf = BytesMut::new();
    map.to_sql(&mut buf).ok();
    let data = buf.to_vec();

    c.bench_function("hstore_decode_10pairs", |b| {
        b.iter(|| {
            let val: std::collections::HashMap<String, Option<String>> =
                FromSql::from_sql(black_box(&data)).expect("decode");
            black_box(val);
        });
    });
}

criterion_group!(
    benches,
    bench_i32_encode,
    bench_i32_decode,
    bench_string_encode,
    bench_uuid_encode,
    bench_uuid_decode,
    bench_oid_lookup,
    bench_hstore_encode,
    bench_hstore_decode,
);
criterion_main!(benches);
