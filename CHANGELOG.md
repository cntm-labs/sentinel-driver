# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- PostgreSQL wire protocol implementation (frontend + backend messages)
- Connection configuration with URL parsing and builder pattern
- SCRAM-SHA-256 and MD5 authentication
- Binary type encoding/decoding for all common PG types
- Pipeline mode for query batching
- COPY IN/OUT protocol (binary and text formats)
- LISTEN/NOTIFY engine with broadcast dispatcher
- Two-tier prepared statement cache (HashMap + LRU)
- Connection pool with health checking
- TLS support via rustls
- FromRow, ToSql, FromSql derive macros
