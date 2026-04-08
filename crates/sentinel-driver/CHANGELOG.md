# Changelog

## [0.1.1](https://github.com/cntm-labs/sentinel-driver/compare/sentinel-driver-v0.1.0...sentinel-driver-v0.1.1) (2026-04-08)


### Features

* add Deref/DerefMut on PooledConnection ([#17](https://github.com/cntm-labs/sentinel-driver/issues/17)) ([dba7483](https://github.com/cntm-labs/sentinel-driver/commit/dba74836951d5b2027d45ffca5a8e37b29a727c1))
* add Deref/DerefMut on PooledConnection for query access ([#17](https://github.com/cntm-labs/sentinel-driver/issues/17)) ([e36aa10](https://github.com/cntm-labs/sentinel-driver/commit/e36aa10e89357cc02659a096b4c0754755977c7d))
* add pool lifecycle callbacks and connect_lazy ([125a03c](https://github.com/cntm-labs/sentinel-driver/commit/125a03c4bf4db9d70bd0bbfc1e6b6b5424282b15))
* add pool lifecycle callbacks and connect_lazy ([92535f8](https://github.com/cntm-labs/sentinel-driver/commit/92535f8956af8a0d0d9f5d124bee159f7eb58092))
* complete all remaining phases (1C, 2C, 3A, 3B, 3C) for v0.1.1 ([d0293a6](https://github.com/cntm-labs/sentinel-driver/commit/d0293a65d7c04b6380c376892a47883c378093f4))
* complete all remaining phases for v0.1.1 ([62df7b3](https://github.com/cntm-labs/sentinel-driver/commit/62df7b320c958b469ccc590f15dd2fcbee971529))
* **stream:** add RowStream for row-by-row query streaming ([a4423ab](https://github.com/cntm-labs/sentinel-driver/commit/a4423ab6513f98a3dec2121c752b4cd32973c472))
* **stream:** add RowStream for row-by-row query streaming (#Phase2A) ([36d9b94](https://github.com/cntm-labs/sentinel-driver/commit/36d9b9410b8131fb32a7de35f51fb6339d31ea23))
* **types:** add BIT/VARBIT support with PgBit struct (#Phase1C) ([a27ecb6](https://github.com/cntm-labs/sentinel-driver/commit/a27ecb6ebb3e9ee25ceae357a35077df9ff40725))
* **types:** Phase 1A core type expansion — 34 new OIDs ([#13](https://github.com/cntm-labs/sentinel-driver/issues/13)) ([38ea74f](https://github.com/cntm-labs/sentinel-driver/commit/38ea74fb9344f865ba1a43d6919f7cdceba7ea32))


### Bug Fixes

* encode NULL params as None instead of empty bytes ([#20](https://github.com/cntm-labs/sentinel-driver/issues/20)) ([aa09a6e](https://github.com/cntm-labs/sentinel-driver/commit/aa09a6e57aa3c489cd8cd9824660b783037f48b1))
* encode NULL params as None instead of empty bytes ([#20](https://github.com/cntm-labs/sentinel-driver/issues/20)) ([e5f2d22](https://github.com/cntm-labs/sentinel-driver/commit/e5f2d22e0906042af6b0015c7482023ffb85fb08))
