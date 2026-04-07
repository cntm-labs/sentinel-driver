use sentinel_driver::Config;

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

#[tokio::test]
async fn test_query_stream_basic() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(config).await.unwrap();

    // Create temp table with test data
    conn.simple_query("CREATE TEMP TABLE stream_test (id INT, name TEXT)")
        .await
        .unwrap();
    conn.execute(
        "INSERT INTO stream_test VALUES ($1, $2), ($3, $4), ($5, $6)",
        &[&1i32, &"Alice", &2i32, &"Bob", &3i32, &"Charlie"],
    )
    .await
    .unwrap();

    // Stream all rows
    let mut stream = conn
        .query_stream("SELECT id, name FROM stream_test ORDER BY id", &[])
        .await
        .unwrap();

    let row = stream.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i32>(0), 1);
    assert_eq!(row.get::<String>(1), "Alice");

    let row = stream.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i32>(0), 2);
    assert_eq!(row.get::<String>(1), "Bob");

    let row = stream.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i32>(0), 3);
    assert_eq!(row.get::<String>(1), "Charlie");

    // Stream exhausted
    assert!(stream.next().await.unwrap().is_none());
    // Repeated call after exhaustion still returns None
    assert!(stream.next().await.unwrap().is_none());

    conn.close().await.unwrap();
}

#[tokio::test]
async fn test_query_stream_empty_result() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(config).await.unwrap();

    conn.simple_query("CREATE TEMP TABLE stream_empty (id INT)")
        .await
        .unwrap();

    let mut stream = conn
        .query_stream("SELECT id FROM stream_empty", &[])
        .await
        .unwrap();

    // No rows — immediately returns None
    assert!(stream.next().await.unwrap().is_none());
    assert!(stream.is_done());

    conn.close().await.unwrap();
}

#[tokio::test]
async fn test_query_stream_with_params() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(config).await.unwrap();

    conn.simple_query("CREATE TEMP TABLE stream_params (id INT, active BOOL)")
        .await
        .unwrap();
    conn.execute(
        "INSERT INTO stream_params VALUES ($1, $2), ($3, $4), ($5, $6)",
        &[&1i32, &true, &2i32, &false, &3i32, &true],
    )
    .await
    .unwrap();

    let mut stream = conn
        .query_stream(
            "SELECT id FROM stream_params WHERE active = $1 ORDER BY id",
            &[&true],
        )
        .await
        .unwrap();

    let row = stream.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i32>(0), 1);

    let row = stream.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i32>(0), 3);

    assert!(stream.next().await.unwrap().is_none());

    conn.close().await.unwrap();
}

#[tokio::test]
async fn test_query_stream_close_early() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(config).await.unwrap();

    conn.simple_query("CREATE TEMP TABLE stream_close (id INT)")
        .await
        .unwrap();
    conn.execute(
        "INSERT INTO stream_close SELECT generate_series(1, 100)",
        &[],
    )
    .await
    .unwrap();

    // Stream some rows, then close early
    let mut stream = conn
        .query_stream("SELECT id FROM stream_close ORDER BY id", &[])
        .await
        .unwrap();

    let row = stream.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i32>(0), 1);

    // Close without consuming remaining 99 rows
    stream.close().await.unwrap();

    // Connection should be reusable after close
    let rows = conn.query("SELECT 42 AS answer", &[]).await.unwrap();
    assert_eq!(rows[0].get::<i32>(0), 42);

    conn.close().await.unwrap();
}

#[tokio::test]
async fn test_query_stream_connection_reuse_after_full_consume() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(config).await.unwrap();

    conn.simple_query("CREATE TEMP TABLE stream_reuse (id INT)")
        .await
        .unwrap();
    conn.execute("INSERT INTO stream_reuse VALUES ($1)", &[&1i32])
        .await
        .unwrap();

    // First stream — fully consumed
    let mut stream = conn
        .query_stream("SELECT id FROM stream_reuse", &[])
        .await
        .unwrap();
    let row = stream.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i32>(0), 1);
    assert!(stream.next().await.unwrap().is_none());
    drop(stream);

    // Second stream on same connection
    let mut stream = conn
        .query_stream("SELECT id FROM stream_reuse", &[])
        .await
        .unwrap();
    let row = stream.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i32>(0), 1);
    assert!(stream.next().await.unwrap().is_none());

    conn.close().await.unwrap();
}

#[tokio::test]
async fn test_query_stream_description() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(config).await.unwrap();

    let mut stream = conn
        .query_stream("SELECT 1 AS num, 'hello'::TEXT AS greeting", &[])
        .await
        .unwrap();

    let desc = stream.description();
    assert_eq!(desc.len(), 2);
    assert_eq!(desc.column_index("num"), Some(0));
    assert_eq!(desc.column_index("greeting"), Some(1));

    // Consume the stream
    while stream.next().await.unwrap().is_some() {}

    conn.close().await.unwrap();
}

#[tokio::test]
async fn test_query_stream_error_invalid_sql() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(config).await.unwrap();

    let result = conn
        .query_stream("SELECT * FROM nonexistent_table_xyz", &[])
        .await;

    assert!(result.is_err());

    // Connection should still be usable after error
    let rows = conn.query("SELECT 1 AS ok", &[]).await.unwrap();
    assert_eq!(rows[0].get::<i32>(0), 1);

    conn.close().await.unwrap();
}

#[tokio::test]
async fn test_query_stream_non_select_error() {
    let url = require_pg!();
    let config = Config::parse(&url).unwrap();
    let mut conn = sentinel_driver::Connection::connect(config).await.unwrap();

    // INSERT doesn't return rows — query_stream should error
    conn.simple_query("CREATE TEMP TABLE stream_noselect (id INT)")
        .await
        .unwrap();

    let result = conn
        .query_stream("INSERT INTO stream_noselect VALUES (1)", &[])
        .await;

    assert!(result.is_err());

    // Connection should still be usable
    let rows = conn.query("SELECT 1 AS ok", &[]).await.unwrap();
    assert_eq!(rows[0].get::<i32>(0), 1);

    conn.close().await.unwrap();
}
