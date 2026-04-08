use sentinel_driver::pool::config::PoolConfig;
use sentinel_driver::pool::Pool;
use sentinel_driver::PooledConnection;

#[test]
fn test_pooled_connection_re_export() {
    // Verify PooledConnection is importable from crate root
    fn _assert_type(_: &PooledConnection) {}
}

#[test]
fn test_pooled_connection_deref_compiles() {
    // Verify Deref<Target=Connection> gives access to Connection methods.
    // We can't call query() without a real PG, but we can verify the
    // method exists through type inference.
    fn _assert_has_is_broken(conn: &PooledConnection) -> bool {
        conn.is_broken() // calls Connection::is_broken via Deref
    }
}

#[test]
fn test_pooled_connection_deref_mut_compiles() {
    // Verify DerefMut gives mutable access to Connection methods.
    fn _assert_has_mark_broken(conn: &mut PooledConnection) {
        conn.mark_broken(); // calls PooledConnection::mark_broken directly
    }
}

#[test]
fn test_connect_lazy_pool_has_max_connections() {
    let config =
        sentinel_driver::Config::parse("postgres://user:pass@localhost/db").expect("valid config");
    let pool = Pool::connect_lazy(config, PoolConfig::new().max_connections(5));
    assert_eq!(pool.max_connections(), 5);
}
