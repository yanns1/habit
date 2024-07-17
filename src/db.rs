use crate::habit::{At, Day};
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

pub fn habit_update_name(
    conn: &Connection,
    habit_name: &str,
    new_name: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE habit SET name = ?1 WHERE name = ?2",
        rusqlite::params![new_name, habit_name],
    )
    .with_context(|| {
        format!(
            "Failed to update name of habit '{}' to '{}'.",
            habit_name, new_name
        )
    })?;

    Ok(())
}

pub fn habit_update_description(
    conn: &Connection,
    habit_name: &str,
    new_description: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE habit SET description = ?1 WHERE name = ?2",
        rusqlite::params![new_description, habit_name],
    )
    .with_context(|| {
        format!(
            "Failed to update description of habit '{}' to '{}'.",
            habit_name, new_description
        )
    })?;

    Ok(())
}

pub fn habit_update_days(
    conn: &Connection,
    habit_name: &str,
    new_days: &[Day],
) -> anyhow::Result<()> {
    let new_days_str = new_days
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<String>>()
        .join(" ");

    conn.execute(
        "UPDATE habit SET days = ?1 WHERE name = ?2",
        rusqlite::params![new_days_str, habit_name],
    )
    .with_context(|| {
        format!(
            "Failed to update days of habit '{}' to '{}'.",
            habit_name, new_days_str
        )
    })?;

    Ok(())
}

pub fn habit_update_at(conn: &Connection, habit_name: &str, new_at: &At) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE habit SET hour = ?1, minutes = ?2 WHERE name = ?3",
        rusqlite::params![new_at.hour, new_at.minutes, habit_name],
    )
    .with_context(|| {
        format!(
            "Failed to update at of habit '{}' to '{}'.",
            habit_name, new_at
        )
    })?;

    Ok(())
}

pub trait DbMapped {
    fn create_table(conn: &Connection) -> anyhow::Result<()>;
    fn insert(&self, conn: &Connection) -> anyhow::Result<()>;
}
