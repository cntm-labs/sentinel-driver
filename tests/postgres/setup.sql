-- Setup SQL for integration tests
-- Run against a fresh PostgreSQL database before running tests

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Test table for basic CRUD operations
CREATE TABLE IF NOT EXISTS test_basic (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    value INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Test table for type coverage
CREATE TABLE IF NOT EXISTS test_types (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    bool_col BOOLEAN,
    int2_col SMALLINT,
    int4_col INTEGER,
    int8_col BIGINT,
    float4_col REAL,
    float8_col DOUBLE PRECISION,
    text_col TEXT,
    bytea_col BYTEA,
    date_col DATE,
    time_col TIME,
    timestamp_col TIMESTAMP,
    timestamptz_col TIMESTAMPTZ
);

-- Test table for COPY operations
CREATE TABLE IF NOT EXISTS test_copy (
    id INTEGER,
    name TEXT,
    score INTEGER
);
