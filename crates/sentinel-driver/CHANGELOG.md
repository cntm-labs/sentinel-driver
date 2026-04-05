# Changelog

## [0.1.1](https://github.com/cntm-labs/sentinel-driver/compare/sentinel-driver-v0.1.0...sentinel-driver-v0.1.1) (2026-04-05)


### Features

* cancel query + per-query timeout (Phase 2) ([994f6eb](https://github.com/cntm-labs/sentinel-driver/commit/994f6ebd2b27fc2a2cb12785f98987c04c6b89f0))
* **cancel:** add CancelToken with cancel() method ([#4](https://github.com/cntm-labs/sentinel-driver/issues/4)) ([12739e5](https://github.com/cntm-labs/sentinel-driver/commit/12739e58ec915c6f849b065bec3cdfd7f24cec98))
* **config:** activate statement_timeout config option ([#3](https://github.com/cntm-labs/sentinel-driver/issues/3)) ([454eafe](https://github.com/cntm-labs/sentinel-driver/commit/454eafe467a715fbe50957d863eb2a227241e5f3))
* **connection:** add cancel_token() method, activate secret_key ([#4](https://github.com/cntm-labs/sentinel-driver/issues/4)) ([ac75ae7](https://github.com/cntm-labs/sentinel-driver/commit/ac75ae78e8ff2cc91723e5a54f4b214ee3b23b82))
* **connection:** add query_with_timeout and execute_with_timeout ([#3](https://github.com/cntm-labs/sentinel-driver/issues/3)) ([7a778f9](https://github.com/cntm-labs/sentinel-driver/commit/7a778f9bdea6d04427cead05dce861857b52318f))
* **connection:** integrate default query timeout from config ([#3](https://github.com/cntm-labs/sentinel-driver/issues/3)) ([650b18b](https://github.com/cntm-labs/sentinel-driver/commit/650b18becc10c60aa7b978f6d3335f0c4f34a81b))
* Phase 1 — array types and health check ([cbe84be](https://github.com/cntm-labs/sentinel-driver/commit/cbe84bec81f828d7e5a5dcd58da8a1669460eea4))
* **pool:** implement Query health check strategy ([#2](https://github.com/cntm-labs/sentinel-driver/issues/2)) ([05cae88](https://github.com/cntm-labs/sentinel-driver/commit/05cae88e7af29c4686791df3b724ad9fa4a0faff))
* **protocol:** add cancel_request message encoder ([#4](https://github.com/cntm-labs/sentinel-driver/issues/4)) ([45ec217](https://github.com/cntm-labs/sentinel-driver/commit/45ec217ec7b4c8f15f8ec65a55b678a56fe2a8df))
* **types:** add FromSql for Vec&lt;T&gt; array decoding ([#5](https://github.com/cntm-labs/sentinel-driver/issues/5)) ([238c03f](https://github.com/cntm-labs/sentinel-driver/commit/238c03fb277c27f07169596e4684a1428b8c3b96))
* **types:** add ToSql for Vec&lt;T&gt; array encoding ([#5](https://github.com/cntm-labs/sentinel-driver/issues/5)) ([60020f2](https://github.com/cntm-labs/sentinel-driver/commit/60020f2282f714e27937c6a606c5af53fde2db2e))


### Bug Fixes

* add per-crate README files instead of relative root path ([55af684](https://github.com/cntm-labs/sentinel-driver/commit/55af684a7d478449665f66fa84780ce30ebb9f35))
* add version to sentinel-derive dependency for crates.io publish ([0a3dbe0](https://github.com/cntm-labs/sentinel-driver/commit/0a3dbe086ec866551f45e487050425c6c563a281))
* **ci:** add PostgreSQL to coverage workflow + pool integration tests ([8978571](https://github.com/cntm-labs/sentinel-driver/commit/897857170ee26cedcd52a37aaf113a6c93f4ab6c))
