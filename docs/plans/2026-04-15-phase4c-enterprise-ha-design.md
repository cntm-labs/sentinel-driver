# Phase 4C: Enterprise HA — Multi-host, Session Routing, Unix Sockets

**Date:** 2026-04-15
**Status:** Approved
**Goal:** Add enterprise deployment features for HA PostgreSQL setups.

## 1. Multi-host Failover

Config stores `Vec<(String, u16)>` for host/port pairs. Connect tries each until success.

### Connection String
```
postgres://user:pass@host1:5432,host2:5433,host3:5432/db?load_balance_hosts=random
```

### Builder
```rust
Config::builder()
    .host("primary.pg.example.com")
    .host("replica1.pg.example.com")
    .port(5432)
    .load_balance_hosts(LoadBalanceHosts::Random)
```

### Enum
```rust
pub enum LoadBalanceHosts {
    Disable,   // try hosts in order (default)
    Random,    // shuffle before trying
}
```

### Connect Logic
```
hosts = config.hosts()  // Vec<(host, port)>
if load_balance == Random { shuffle(hosts) }
for (host, port) in hosts {
    match try_connect(host, port, tls).await {
        Ok(conn) => {
            if check_session_attrs(conn, target) { return Ok(conn) }
            conn.close()  // wrong session attrs, try next
        }
        Err(_) => continue  // connection failed, try next
    }
}
Err(Error::AllHostsFailed)
```

## 2. target_session_attrs

Route connections to primary or replica based on read/write capability.

```rust
pub enum TargetSessionAttrs {
    Any,        // accept any server (default)
    ReadWrite,  // primary only (transaction_read_only = off)
    ReadOnly,   // replica only (transaction_read_only = on)
}
```

### Check Logic (after successful auth)
```rust
let row = conn.simple_query("SHOW transaction_read_only").await?;
let is_read_only = row == "on";
match target {
    Any => Ok(()),
    ReadWrite if !is_read_only => Ok(()),
    ReadOnly if is_read_only => Ok(()),
    _ => Err(Error::WrongSessionAttrs),
}
```

### Connection String
```
postgres://user@host/db?target_session_attrs=read-write
```

## 3. Unix Domain Socket

Detect path-based host (starts with `/`) and use `UnixStream`.

### Connection String
```
postgres://user@/db?host=/var/run/postgresql
```

### Socket Path
`{host}/.s.PGSQL.{port}` (e.g., `/var/run/postgresql/.s.PGSQL.5432`)

### Implementation
- `PgStream` enum gets a third variant: `Unix(UnixStream)`
- Feature-gated: `#[cfg(unix)]` (not available on Windows)
- `config.rs` detects `host.starts_with('/')` to select Unix path

## Files

| File | Changes |
|------|---------|
| `config.rs` | Multi-host Vec, LoadBalanceHosts, TargetSessionAttrs, Unix host detection, connection string parsing |
| `connection/stream.rs` | Connect loop over hosts, UnixStream variant, session attrs check |
| `connection/startup.rs` | Post-auth session attrs verification |
| `error.rs` | AllHostsFailed, WrongSessionAttrs error variants |

## Dependencies
None new — `tokio::net::UnixStream` is in tokio with `net` feature (already enabled).
