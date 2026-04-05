/// Transaction isolation levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl IsolationLevel {
    pub fn as_sql(&self) -> &'static str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED",
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
}

/// Transaction access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    ReadWrite,
    ReadOnly,
}

/// Configuration for beginning a transaction.
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub(crate) isolation: Option<IsolationLevel>,
    pub(crate) access_mode: Option<AccessMode>,
    pub(crate) deferrable: bool,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionConfig {
    pub fn new() -> Self {
        Self {
            isolation: None,
            access_mode: None,
            deferrable: false,
        }
    }

    pub fn isolation(mut self, level: IsolationLevel) -> Self {
        self.isolation = Some(level);
        self
    }

    pub fn read_only(mut self) -> Self {
        self.access_mode = Some(AccessMode::ReadOnly);
        self
    }

    pub fn read_write(mut self) -> Self {
        self.access_mode = Some(AccessMode::ReadWrite);
        self
    }

    pub fn deferrable(mut self, deferrable: bool) -> Self {
        self.deferrable = deferrable;
        self
    }

    /// Build the BEGIN statement SQL.
    pub fn begin_sql(&self) -> String {
        let mut sql = String::from("BEGIN");
        let mut has_option = false;

        if let Some(isolation) = &self.isolation {
            sql.push_str(" ISOLATION LEVEL ");
            sql.push_str(isolation.as_sql());
            has_option = true;
        }

        if let Some(access) = &self.access_mode {
            if has_option {
                sql.push(',');
            }
            match access {
                AccessMode::ReadWrite => sql.push_str(" READ WRITE"),
                AccessMode::ReadOnly => sql.push_str(" READ ONLY"),
            }
            has_option = true;
        }

        if self.deferrable {
            if has_option {
                sql.push(',');
            }
            sql.push_str(" DEFERRABLE");
        }

        sql
    }
}
