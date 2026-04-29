//! Live-PostgreSQL coverage for nullable array decoding (issue #33).
//!
//! Verifies that `Vec<Option<T>>` round-trips against the server for the
//! types listed on the issue. Each test runs only when `DATABASE_URL` is
//! set, mirroring the convention used by the rest of `tests/postgres/`.

use sentinel_driver::{Config, Connection};

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

macro_rules! require_pg {
    () => {
        match database_url() {
            Some(url) => url,
            None => return,
        }
    };
}

async fn connect() -> Connection {
    let url = database_url().expect("require_pg! gates this");
    let config = Config::parse(&url).unwrap();
    Connection::connect(config).await.unwrap()
}

#[tokio::test]
async fn test_int4_array_with_null() {
    let _ = require_pg!();
    let mut conn = connect().await;

    let rows = conn
        .query("SELECT ARRAY[1, NULL, 3]::int4[]", &[])
        .await
        .unwrap();

    let got: Vec<Option<i32>> = rows[0].get(0);
    assert_eq!(got, vec![Some(1), None, Some(3)]);
}

#[tokio::test]
async fn test_text_array_with_null() {
    let _ = require_pg!();
    let mut conn = connect().await;

    let rows = conn
        .query("SELECT ARRAY['a', NULL, 'c']::text[]", &[])
        .await
        .unwrap();

    let got: Vec<Option<String>> = rows[0].get(0);
    assert_eq!(
        got,
        vec![Some("a".to_string()), None, Some("c".to_string())]
    );
}

#[tokio::test]
async fn test_bool_array_with_null() {
    let _ = require_pg!();
    let mut conn = connect().await;

    let rows = conn
        .query("SELECT ARRAY[true, NULL, false]::bool[]", &[])
        .await
        .unwrap();

    let got: Vec<Option<bool>> = rows[0].get(0);
    assert_eq!(got, vec![Some(true), None, Some(false)]);
}

#[tokio::test]
async fn test_int4_array_no_nulls_via_option() {
    // `Vec<Option<T>>` must also accept arrays that happen to have no NULLs.
    let _ = require_pg!();
    let mut conn = connect().await;

    let rows = conn.query("SELECT ARRAY[1, 2, 3]::int4[]", &[]).await.unwrap();

    let got: Vec<Option<i32>> = rows[0].get(0);
    assert_eq!(got, vec![Some(1), Some(2), Some(3)]);
}

#[tokio::test]
async fn test_int4_array_all_null() {
    let _ = require_pg!();
    let mut conn = connect().await;

    let rows = conn
        .query(
            "SELECT ARRAY[NULL, NULL, NULL]::int4[]",
            &[],
        )
        .await
        .unwrap();

    let got: Vec<Option<i32>> = rows[0].get(0);
    assert_eq!(got, vec![None, None, None]);
}

#[tokio::test]
async fn test_int4_array_with_null_rejected_by_non_option_vec() {
    // The historical `Vec<i32>` decode path must keep erroring on NULL,
    // and the error message must point at `Vec<Option<T>>`.
    let _ = require_pg!();
    let mut conn = connect().await;

    let rows = conn
        .query("SELECT ARRAY[1, NULL, 3]::int4[]", &[])
        .await
        .unwrap();

    let result = std::panic::catch_unwind(|| {
        let _: Vec<i32> = rows[0].get(0);
    });
    assert!(
        result.is_err(),
        "Vec<i32> decode of an array containing NULL must panic"
    );
}

#[tokio::test]
async fn test_uuid_array_with_null() {
    let _ = require_pg!();
    let mut conn = connect().await;

    let rows = conn
        .query(
            "SELECT ARRAY[\
             '550e8400-e29b-41d4-a716-446655440000'::uuid, \
             NULL, \
             '00000000-0000-0000-0000-000000000000'::uuid\
             ]::uuid[]",
            &[],
        )
        .await
        .unwrap();

    let got: Vec<Option<uuid::Uuid>> = rows[0].get(0);
    assert_eq!(got.len(), 3);
    assert_eq!(
        got[0],
        Some(uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap())
    );
    assert_eq!(got[1], None);
    assert_eq!(got[2], Some(uuid::Uuid::nil()));
}
