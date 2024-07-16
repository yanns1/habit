use crate::DB_PATH;
use anyhow::Context;
use rusqlite::Connection;

pub fn open_db() -> anyhow::Result<Connection> {
    Connection::open(DB_PATH.clone()).with_context(|| {
        format!(
            "Failed to open sqlite db file at location {}",
            DB_PATH.to_string_lossy()
        )
    })
}
