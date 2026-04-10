# Phase 3A Completion: Client Certs, Direct TLS, SCRAM-PLUS

**Date:** 2026-04-10
**Status:** Approved
**Goal:** Wire up the 3 remaining TLS/auth features that have config fields but no runtime implementation.

## Item 1: Client Certificate Authentication

### Current State
- `Config` has `ssl_client_cert: Option<PathBuf>` and `ssl_client_key: Option<PathBuf>` fields
- `tls/mod.rs` uses `.with_no_client_auth()` in both code paths
- ConfigBuilder has no setter methods for these fields

### Changes
- **`tls/mod.rs`**: When `ssl_client_cert` + `ssl_client_key` are set:
  - Read PEM files with `rustls_pemfile::certs()` and `rustls_pemfile::private_key()`
  - Use `.with_client_auth_cert(cert_chain, key)` instead of `.with_no_client_auth()`
  - If only one of cert/key is set, return config error
- **`config.rs`**: Add `.ssl_client_cert(path)` and `.ssl_client_key(path)` builder methods
- **`config.rs`**: Parse `sslcert` and `sslkey` from connection string query params
- **`Cargo.toml`**: Add `rustls-pemfile = "2"` dependency

### Error Cases
- Cert file not found → `Error::Tls("client certificate file not found")`
- Key file not found → `Error::Tls("client key file not found")`
- Invalid PEM format → `Error::Tls("invalid certificate/key PEM format")`
- Cert without key (or vice versa) → `Error::Config("ssl_client_cert requires ssl_client_key")`

## Item 2: Direct TLS (PG 17+)

### Current State
- `Config` has `ssl_direct: bool` field (default: false)
- `connection/stream.rs` always sends SSLRequest before TLS handshake

### Changes
- **`connection/stream.rs`**: Before SSLRequest flow, check `config.ssl_direct()`:
  - If true + TLS config present → perform TLS handshake immediately on raw TCP socket
  - Skip SSLRequest message and response byte entirely
  - Set ALPN protocol to `"postgresql"` via rustls ClientConfig
- **`config.rs`**: Add `.ssl_direct(bool)` builder method
- **`config.rs`**: Parse `ssldirect=true` from connection string (or `sslnegotiation=direct`)

### Error Cases
- `ssl_direct = true` but `ssl_mode = Disable` → `Error::Config("ssl_direct requires TLS")`
- TLS handshake fails on direct connect → `Error::Tls(...)` (same as normal TLS failure)

## Item 3: SCRAM-SHA-256-PLUS (Channel Binding)

### Current State
- `Config` has `channel_binding: ChannelBinding` enum (Prefer/Require/Disable)
- `auth/scram.rs` hardcodes GS2 header `"n,,"` (no channel binding)
- SCRAM always sends mechanism `SCRAM-SHA-256`, never `SCRAM-SHA-256-PLUS`

### Changes
- **`auth/scram.rs`**: Make channel binding conditional:
  - `Disable` or no TLS → GS2 header `"n,,"`, mechanism `SCRAM-SHA-256`
  - `Prefer` + TLS available → try `SCRAM-SHA-256-PLUS` if server offers it, fallback to `SCRAM-SHA-256`
  - `Require` + TLS → must use `SCRAM-SHA-256-PLUS`, error if server doesn't offer it
  - `Require` + no TLS → return `Error::Auth("channel binding requires TLS")`
- **GS2 header for PLUS**: `"p=tls-server-end-point,,"`
- **Channel binding data**: SHA-256 hash of server's DER-encoded certificate
  - Extract from TLS session via `rustls::Connection::peer_certificates()`
  - Compute `sha2::Sha256::digest(der_cert)` → 32 bytes
  - Append to `c=` parameter: `base64(gs2_header + cbind_data)`
- **`connection/stream.rs`**: Add method to extract server certificate from TLS session
- **`config.rs`**: Add `.channel_binding(ChannelBinding)` builder method

### Protocol Detail
```
# Without channel binding (current):
client-first: n,,n=user,r=nonce
client-final: c=biws,r=combined_nonce,p=proof
  (biws = base64("n,,"))

# With tls-server-end-point channel binding:
client-first: p=tls-server-end-point,,n=user,r=nonce
client-final: c=cCF0bHMtc2VydmVyLWVuZC1wb2ludCwsXYZ...,r=combined_nonce,p=proof
  (base64("p=tls-server-end-point,," + sha256(server_cert_der)))
```

## Files to Modify

| File | Changes |
|------|---------|
| `tls/mod.rs` | Load client certs, ALPN for direct TLS, extract server cert |
| `auth/scram.rs` | Conditional GS2 header, channel binding data, mechanism selection |
| `connection/stream.rs` | Direct TLS bypass, expose server cert extraction |
| `config.rs` | Builder methods, connection string parsing |
| `Cargo.toml` | Add `rustls-pemfile = "2"` |

## New Dependencies
- `rustls-pemfile = "2"` — PEM file parsing for client certificates

## Test Strategy
- Unit tests for PEM loading (valid/invalid/missing files)
- Unit tests for GS2 header construction with/without channel binding
- Unit tests for direct TLS config validation
- Integration tests require PG server with cert auth enabled (existing docker-compose can be extended)
