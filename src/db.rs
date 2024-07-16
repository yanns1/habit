use rusqlite::Connection;

pub trait DbMapped {
    fn create_table(conn: &Connection) -> anyhow::Result<()>;
    fn insert(&self, conn: &Connection) -> anyhow::Result<()>;
}
