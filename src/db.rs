use std::str::FromStr;

use crate::habit::{At, Day, Habit};
use crate::DB_PATH;
use anyhow::anyhow;
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

pub fn habit_create_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "CREATE TABLE habit (
            name        TEXT PRIMARY KEY,
            description TEXT NOT NULL,
            days        TEXT NOT NULL,
            hour        INTEGER NOT NULL,
            minutes     INTEGER NOT NULL
        )",
        (),
    )
    .with_context(|| "Failed to create habit table.")?;

    Ok(())
}

pub fn habit_insert(conn: &Connection, habit: &Habit) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO habit (name, description, days, hour, minutes) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            habit.name,
            habit.description,
            habit
                .days
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<String>>()
                .join(" "),
            habit.at.hour,
            habit.at.minutes,
        ],
    )
    .with_context(|| "Failed to insert habit into database.")?;

    Ok(())
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

pub fn habit_exists(conn: &Connection, habit_name: &str) -> anyhow::Result<bool> {
    match conn.query_row(
        "SELECT name FROM habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |_| Ok(()),
    ) {
        Ok(_) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(anyhow!(
            "Query to select habit with name '{}' failed.\n{}",
            habit_name,
            e
        )),
    }
}

pub fn habit_get_by_name(conn: &Connection, habit_name: &str) -> anyhow::Result<Habit> {
    let query_res = conn.query_row(
        "SELECT name, description, days, hour, minutes FROM habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |row| {
            let days: Vec<Day> = row
                .get::<usize, String>(2)?
                .split(' ')
                .map(|d_str| Day::from_str(d_str).expect("Days from database should be valid."))
                .collect();
            let at = At::build(row.get::<usize, u8>(3)?, row.get::<usize, u8>(4)?)
                .expect("Hour and minutes from database should be valid.");
            Ok(Habit::new(
                row.get::<usize, String>(0)?,
                row.get::<usize, String>(1)?,
                days,
                at,
            ))
        },
    );

    match query_res {
        Ok(habit) => Ok(habit),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(anyhow!("Habit '{}' does not exists!", habit_name))
        }
        Err(e) => Err(anyhow!(
            "Query to select habit with name '{}' failed.\n{}",
            habit_name,
            e
        )),
    }
}

pub fn log_create_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "CREATE TABLE log (
            id        INTEGER PRIMARY KEY,
            habit     TEXT NOT NULL REFERENCES habit(name) ON DELETE CASCADE,
            created   TEXT NOT NULL
        )",
        (),
    )
    .with_context(|| "Failed to create log table.")?;

    Ok(())
}

pub fn log_insert(conn: &Connection, habit: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO log (habit, created) VALUES (?1, ?2);",
        rusqlite::params![habit, chrono::Local::now().to_rfc3339()],
    )
    .with_context(|| "Failed to insert log into database.")?;

    Ok(())
}

pub fn get_n_logs_for_habit(conn: &Connection, habit: &str) -> anyhow::Result<usize> {
    conn.query_row(
        "SELECT COUNT(*) FROM log WHERE habit = ?1",
        rusqlite::params![habit],
        |row| row.get::<usize, usize>(0),
    )
    .with_context(|| {
        format!(
            "Failed to count number of logged reps for habit '{}'.",
            habit
        )
    })
}
