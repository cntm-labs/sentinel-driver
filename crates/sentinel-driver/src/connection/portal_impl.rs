use super::Connection;
use crate::error::Result;
use crate::portal::{self, Portal};
use crate::row::Row;
use crate::types::ToSql;

impl Connection {
    /// Create a server-side portal (cursor) for incremental row fetching.
    ///
    /// Portals only work inside transactions. The portal borrows the
    /// connection state, so no concurrent queries while a portal is open.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example(conn: &mut sentinel_driver::Connection) -> sentinel_driver::Result<()> {
    /// conn.begin().await?;
    /// let portal = conn.bind_portal("SELECT * FROM big_table", &[]).await?;
    /// let batch = conn.query_portal(&portal, 100).await?;
    /// conn.close_portal(portal).await?;
    /// conn.commit().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bind_portal(
        &mut self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Portal> {
        portal::create_portal(&mut self.conn, sql, params).await
    }

    /// Fetch up to `max_rows` rows from a portal.
    ///
    /// Returns an empty `Vec` when the cursor is exhausted.
    /// Use `max_rows = 0` to fetch all remaining rows.
    pub async fn query_portal(&mut self, portal: &Portal, max_rows: i32) -> Result<Vec<Row>> {
        let mut portal_mut = Portal {
            name: portal.name.clone(),
            description: portal.description.clone(),
            exhausted: portal.exhausted,
        };
        let rows = portal::fetch_portal(&mut self.conn, &mut portal_mut, max_rows).await?;
        Ok(rows)
    }

    /// Close a portal on the server, freeing resources.
    pub async fn close_portal(&mut self, portal: Portal) -> Result<()> {
        portal::close_portal(&mut self.conn, portal).await
    }
}
