use crate::error::Result;
use crate::row::{Row, SimpleQueryMessage};
use crate::types::ToSql;
use crate::Connection;

/// A trait for types that can execute PostgreSQL queries.
///
/// Allows writing code that is generic over [`Connection`](crate::Connection)
/// and [`PooledConnection`](crate::PooledConnection).
///
/// ```rust,no_run
/// use sentinel_driver::{GenericClient, Result, Row};
///
/// async fn get_user_name(client: &mut impl GenericClient, id: i32) -> Result<String> {
///     let row = client.query_one("SELECT name FROM users WHERE id = $1", &[&id]).await?;
///     row.try_get(0)
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait GenericClient {
    /// Execute a query that returns rows.
    async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>>;

    /// Execute a query that returns a single row.
    async fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row>;

    /// Execute a query that returns an optional single row.
    async fn query_opt(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)])
        -> Result<Option<Row>>;

    /// Execute a non-SELECT query, returning rows affected.
    async fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64>;

    /// Execute a simple query (text protocol, no parameters).
    async fn simple_query(&mut self, sql: &str) -> Result<Vec<SimpleQueryMessage>>;

    /// Execute a typed query (skips the prepare round-trip).
    async fn query_typed(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Vec<Row>>;

    /// Execute a typed query that returns a single row.
    async fn query_typed_one(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Row>;

    /// Execute a typed query that returns an optional row.
    async fn query_typed_opt(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Option<Row>>;

    /// Execute a typed non-SELECT, returning rows affected.
    async fn execute_typed(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<u64>;

    /// Run a pre-built `PipelineBatch` in one round-trip.
    async fn execute_pipeline(
        &mut self,
        batch: crate::pipeline::batch::PipelineBatch,
    ) -> Result<Vec<crate::pipeline::QueryResult>>;
}

impl GenericClient for Connection {
    async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        Connection::query(self, sql, params).await
    }

    async fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        Connection::query_one(self, sql, params).await
    }

    async fn query_opt(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        Connection::query_opt(self, sql, params).await
    }

    async fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        Connection::execute(self, sql, params).await
    }

    async fn simple_query(&mut self, sql: &str) -> Result<Vec<SimpleQueryMessage>> {
        Connection::simple_query(self, sql).await
    }

    async fn query_typed(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Vec<Row>> {
        Connection::query_typed(self, sql, params).await
    }

    async fn query_typed_one(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Row> {
        Connection::query_typed_one(self, sql, params).await
    }

    async fn query_typed_opt(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Option<Row>> {
        Connection::query_typed_opt(self, sql, params).await
    }

    async fn execute_typed(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<u64> {
        Connection::execute_typed(self, sql, params).await
    }

    async fn execute_pipeline(
        &mut self,
        batch: crate::pipeline::batch::PipelineBatch,
    ) -> Result<Vec<crate::pipeline::QueryResult>> {
        Connection::execute_pipeline(self, batch).await
    }
}

impl GenericClient for crate::PooledConnection {
    async fn query(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        Connection::query(self, sql, params).await
    }

    async fn query_one(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        Connection::query_one(self, sql, params).await
    }

    async fn query_opt(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>> {
        Connection::query_opt(self, sql, params).await
    }

    async fn execute(&mut self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        Connection::execute(self, sql, params).await
    }

    async fn simple_query(&mut self, sql: &str) -> Result<Vec<SimpleQueryMessage>> {
        Connection::simple_query(self, sql).await
    }

    async fn query_typed(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Vec<Row>> {
        Connection::query_typed(self, sql, params).await
    }

    async fn query_typed_one(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Row> {
        Connection::query_typed_one(self, sql, params).await
    }

    async fn query_typed_opt(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<Option<Row>> {
        Connection::query_typed_opt(self, sql, params).await
    }

    async fn execute_typed(
        &mut self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), crate::Oid)],
    ) -> Result<u64> {
        Connection::execute_typed(self, sql, params).await
    }

    async fn execute_pipeline(
        &mut self,
        batch: crate::pipeline::batch::PipelineBatch,
    ) -> Result<Vec<crate::pipeline::QueryResult>> {
        Connection::execute_pipeline(self, batch).await
    }
}
