use super::{Connection, Result, copy};

impl Connection {
    /// Start a COPY IN operation for bulk data loading.
    pub async fn copy_in(&mut self, sql: &str) -> Result<copy::CopyIn<'_>> {
        let (format, col_count) = copy::start_copy_in(&mut self.conn, sql).await?;
        Ok(copy::CopyIn::new(&mut self.conn, format, col_count))
    }

    /// Start a COPY OUT operation for bulk data export.
    pub async fn copy_out(&mut self, sql: &str) -> Result<copy::CopyOut<'_>> {
        let format = copy::start_copy_out(&mut self.conn, sql).await?;
        Ok(copy::CopyOut::new(&mut self.conn, format))
    }
}
