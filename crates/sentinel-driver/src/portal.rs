use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::connection::stream::PgConnection;
use crate::error::{Error, Result};
use crate::protocol::backend::BackendMessage;
use crate::protocol::frontend;
use crate::row::{Row, RowDescription};
use crate::types::ToSql;

use bytes::BytesMut;

static PORTAL_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_portal_name() -> String {
    let id = PORTAL_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("_sp{id}")
}

/// A server-side portal (cursor) for incremental row fetching.
///
/// Portals only work inside transactions (PostgreSQL requirement).
/// The portal borrows the connection mutably — no concurrent queries
/// while a portal is open.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
/// conn.begin().await?;
///
/// let portal = conn.bind_portal("SELECT * FROM big_table", &[]).await?;
/// let batch1 = conn.query_portal(&portal, 100).await?; // first 100 rows
/// let batch2 = conn.query_portal(&portal, 100).await?; // next 100
/// // empty Vec = cursor exhausted
///
/// conn.close_portal(portal).await?;
/// conn.commit().await?;
/// # Ok(())
/// # }
/// ```
pub struct Portal {
    pub(crate) name: String,
    pub(crate) description: Arc<RowDescription>,
    pub(crate) exhausted: bool,
}

impl Portal {
    /// Returns `true` if all rows have been fetched.
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    /// The portal name on the server.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Create a portal by parsing+binding a query with named portal.
pub(crate) async fn create_portal(
    conn: &mut PgConnection,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Portal> {
    let portal_name = next_portal_name();

    // Encode parameters
    let param_types: Vec<u32> = params.iter().map(|p| p.oid().0).collect();
    let mut param_data: Vec<Option<&[u8]>> = Vec::with_capacity(params.len());
    let mut param_bufs: Vec<BytesMut> = Vec::with_capacity(params.len());

    for param in params {
        if param.is_null() {
            param_bufs.push(BytesMut::new());
            param_data.push(None);
        } else {
            let mut buf = BytesMut::new();
            param.to_sql(&mut buf)?;
            param_bufs.push(buf);
            // placeholder — will fix below
            param_data.push(None);
        }
    }
    // Build refs from bufs
    let param_refs: Vec<Option<&[u8]>> = params
        .iter()
        .zip(&param_bufs)
        .map(|(p, buf)| {
            if p.is_null() {
                None
            } else {
                Some(buf.as_ref() as &[u8])
            }
        })
        .collect();

    // Parse(unnamed stmt) + Bind(named portal) + Describe(portal) + Sync
    let oids: Vec<u32> = param_types;
    frontend::parse(conn.write_buf(), "", sql, &oids);
    frontend::bind(conn.write_buf(), &portal_name, "", &param_refs, &[]);
    frontend::describe_portal(conn.write_buf(), &portal_name);
    frontend::sync(conn.write_buf());
    conn.send().await?;

    // ParseComplete
    match conn.recv().await? {
        BackendMessage::ParseComplete => {}
        BackendMessage::ErrorResponse { fields } => {
            drain_until_ready(conn).await.ok();
            return Err(Error::server(
                fields.severity,
                fields.code,
                fields.message,
                fields.detail,
                fields.hint,
                fields.position,
            ));
        }
        other => {
            return Err(Error::protocol(format!(
                "portal: expected ParseComplete, got {other:?}"
            )));
        }
    }

    // BindComplete
    match conn.recv().await? {
        BackendMessage::BindComplete => {}
        BackendMessage::ErrorResponse { fields } => {
            drain_until_ready(conn).await.ok();
            return Err(Error::server(
                fields.severity,
                fields.code,
                fields.message,
                fields.detail,
                fields.hint,
                fields.position,
            ));
        }
        other => {
            return Err(Error::protocol(format!(
                "portal: expected BindComplete, got {other:?}"
            )));
        }
    }

    // RowDescription
    let description = match conn.recv().await? {
        BackendMessage::RowDescription { fields } => Arc::new(RowDescription::new(fields)),
        BackendMessage::NoData => Arc::new(RowDescription::new(vec![])),
        other => {
            return Err(Error::protocol(format!(
                "portal: expected RowDescription, got {other:?}"
            )));
        }
    };

    // ReadyForQuery
    drain_until_ready(conn).await?;

    Ok(Portal {
        name: portal_name,
        description,
        exhausted: false,
    })
}

/// Fetch rows from a portal with a maximum row limit.
pub(crate) async fn fetch_portal(
    conn: &mut PgConnection,
    portal: &mut Portal,
    max_rows: i32,
) -> Result<Vec<Row>> {
    if portal.exhausted {
        return Ok(Vec::new());
    }

    frontend::execute(conn.write_buf(), &portal.name, max_rows);
    frontend::sync(conn.write_buf());
    conn.send().await?;

    let mut rows = Vec::new();

    loop {
        match conn.recv().await? {
            BackendMessage::DataRow { columns } => {
                rows.push(Row::new(columns, Arc::clone(&portal.description)));
            }
            BackendMessage::PortalSuspended => {
                // More rows available
                break;
            }
            BackendMessage::CommandComplete { .. } => {
                portal.exhausted = true;
                break;
            }
            BackendMessage::ErrorResponse { fields } => {
                drain_until_ready(conn).await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            _ => {}
        }
    }

    // ReadyForQuery
    drain_until_ready(conn).await?;

    Ok(rows)
}

/// Close a portal on the server.
pub(crate) async fn close_portal(conn: &mut PgConnection, portal: Portal) -> Result<()> {
    if portal.exhausted {
        return Ok(());
    }
    frontend::close_portal(conn.write_buf(), &portal.name);
    frontend::sync(conn.write_buf());
    conn.send().await?;

    // CloseComplete
    loop {
        match conn.recv().await? {
            BackendMessage::CloseComplete => break,
            BackendMessage::ErrorResponse { fields } => {
                drain_until_ready(conn).await.ok();
                return Err(Error::server(
                    fields.severity,
                    fields.code,
                    fields.message,
                    fields.detail,
                    fields.hint,
                    fields.position,
                ));
            }
            _ => {}
        }
    }

    drain_until_ready(conn).await?;
    Ok(())
}

async fn drain_until_ready(conn: &mut PgConnection) -> Result<()> {
    loop {
        if matches!(conn.recv().await?, BackendMessage::ReadyForQuery { .. }) {
            return Ok(());
        }
    }
}
