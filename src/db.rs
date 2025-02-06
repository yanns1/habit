use crate::habit;
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

pub fn create_tables(conn: &Connection) -> anyhow::Result<()> {
    // Use an integer for storing days. Only seven bits are actually useful, one per day.
    // A day's bit should be 1 if it is included, 0 otherwise.
    // -----
    // For `Habit`, I faced a challenge. I want the user to be able to change
    // an habit's name, description or "at". But I also want to be able to show
    // him his progress (see `show` module), i.e. all the logs there are for a
    // given habit, and how they compare to what was planned. Thus, I need to
    // keep track of a habit's *history*. This is a common problem. Articles
    // I found exposing basic solutions are:
    //
    // - <https://www.codeproject.com/Articles/105768/Audit-Trail-Tracing-Data-Changes-in-Database>.
    // - <https://dev.to/zhiyueyi/design-a-table-to-keep-historical-changes-in-database-10fn>
    //
    // I chose the "history table" strategy instead of the "audit table" strategy, because it is
    // simpler. Even though audit tables (if done properly) scale better as the number of updates
    // and database tables grow, they are trickier to query. In my case, I know there will be
    // few updates by the user most of the time, so whatever I do, it will work just fine.
    conn.execute_batch(
        "
        BEGIN;
        CREATE TABLE Habit (
            id          INTEGER NOT NULL UNIQUE,
            name        TEXT NOT NULL,
            description TEXT NOT NULL,
            days        INTEGER NOT NULL,
            hour        INTEGER NOT NULL,
            minutes     INTEGER NOT NULL,
            suspended   INTEGER NOT NULL,
            PRIMARY KEY (name)
        );
        CREATE TABLE HabitHistory (
            habit_id    INTEGER NOT NULL,
            created_at  INTEGER NOT NULL,
            name        TEXT NOT NULL,
            description TEXT NOT NULL,
            days        INTEGER NOT NULL,
            hour        INTEGER NOT NULL,
            minutes     INTEGER NOT NULL,
            suspended   INTEGER NOT NULL,
            PRIMARY KEY (habit_id, created_at),
            FOREIGN KEY (habit_id) REFERENCES Habit(id) ON DELETE CASCADE
        );
        CREATE TABLE Log (
            created_at INTEGER,
            habit_id   INTEGER NOT NULL,
            PRIMARY KEY (created_at),
            FOREIGN KEY (habit_id) REFERENCES Habit(id) ON DELETE CASCADE
        );
        COMMIT;
        ",
    )
    .with_context(|| "Failed to create tables.")?;

    Ok(())
}

pub fn habit_insert(conn: &Connection, habit: &Habit) -> anyhow::Result<()> {
    // Query the current max id to have the new be one more.
    // This can be slow if there are many habits. It is reasonable to
    // assume that there will never be enough in practice to actually make
    // this noticably slow. I thought about using the current number of rows
    // in the table instead (using `COUNT`), but realized that it does not
    // work, because rows can be deleted (see module `delete`).
    //
    // The very first row inserted is a special case. If the table is empty,
    // MAX will fail. So first check if the table is empty. If so, set `id` to 0.
    let is_empty = conn
        .query_row("SELECT EXISTS (SELECT 1 FROM Habit)", [], |row| {
            row.get::<_, u8>(0).map(|v| v == 0)
        })
        .with_context(|| "Query to see if there exists at least one row in Habit failed.")?;
    let id = if is_empty {
        0
    } else {
        conn.query_row("SELECT MAX(id) FROM Habit", [], |row| {
            row.get::<_, usize>(0).map(|id| id + 1)
        })
        .with_context(|| "Query to select max id of Habit failed.")?
    };
    let byte = habit::days_to_byte(&habit.days[..]);

    conn.execute(
        "INSERT INTO Habit (id, name, description, days, hour, minutes, suspended) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            id,
            habit.name,
            habit.description,
            byte,
            habit.at.hour,
            habit.at.minutes,
            habit.suspended,
        ],
    )
    .with_context(|| format!("Failed to insert '({}, {}, {}, {}, {}, {}, {})' into Habit.", id,
        habit.name, habit.description, byte, habit.at.hour, habit.at.minutes, habit.suspended))?;

    Ok(())
}

pub fn habit_write_history(conn: &Connection, habit_name: &str) -> anyhow::Result<()> {
    let (id, name, description, days, hour, minutes, suspended) = conn
        .query_row(
            "SELECT id, name, description, days, hour, minutes, suspended FROM Habit WHERE name = ?1",
            rusqlite::params![habit_name],
            |row| {
                let id = row.get::<_, usize>(0)?;
                let name = row.get::<_, String>(1)?;
                let description = row.get::<_, String>(2)?;
                let days = row.get::<_, u8>(3)?;
                let hour = row.get::<_, u8>(4)?;
                let minutes = row.get::<_, u8>(5)?;
                let suspended = row.get::<_, bool>(6)?;
                Ok((id, name, description, days, hour, minutes, suspended))
            },
        )
        .with_context(|| format!("Failed to select Habit with name '{}'.", habit_name))?;

    let created_at = chrono::Local::now().timestamp();
    conn.execute(
        "INSERT INTO HabitHistory (habit_id, created_at, name, description, days, hour, minutes, suspended) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, created_at, name, description, days, hour, minutes, suspended]
    ).with_context(|| {
        format!("Failed to insert '({}, {}, {}, {}, {}, {}, {}, {})' into HabitHistory.", id, created_at, name, description, days, hour, minutes, suspended)
    })?;

    Ok(())
}

pub fn habit_update_name(
    conn: &Connection,
    habit_name: &str,
    new_name: &str,
) -> anyhow::Result<()> {
    habit_write_history(conn, habit_name)?;

    conn.execute(
        "UPDATE Habit SET name = ?1 WHERE name = ?2",
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
    habit_write_history(conn, habit_name)?;

    conn.execute(
        "UPDATE Habit SET description = ?1 WHERE name = ?2",
        rusqlite::params![new_description, habit_name],
    )
    .with_context(|| {
        format!(
            "Failed to update description of Habit '{}', to '{}'.",
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
    habit_write_history(conn, habit_name)?;

    let byte = habit::days_to_byte(new_days);
    conn.execute(
        "UPDATE Habit SET days = ?1 WHERE name = ?2",
        rusqlite::params![byte, habit_name],
    )
    .with_context(|| {
        format!(
            "Failed to update days of Habit '{}', to '{}'.",
            habit_name, byte
        )
    })?;

    Ok(())
}

pub fn habit_update_at(conn: &Connection, habit_name: &str, new_at: &At) -> anyhow::Result<()> {
    habit_write_history(conn, habit_name)?;

    conn.execute(
        "UPDATE Habit SET hour = ?1, minutes = ?2 WHERE name = ?3",
        rusqlite::params![new_at.hour, new_at.minutes, habit_name],
    )
    .with_context(|| {
        format!(
            "Failed to update at of Habit '{}', to '{}'.",
            habit_name, new_at
        )
    })?;

    Ok(())
}

pub fn habit_update_suspended(
    conn: &Connection,
    habit_name: &str,
    new_suspended: bool,
) -> anyhow::Result<()> {
    habit_write_history(conn, habit_name)?;

    conn.execute(
        "UPDATE Habit SET suspended = ?1 WHERE name = ?2",
        rusqlite::params![new_suspended, habit_name],
    )
    .with_context(|| {
        format!(
            "Failed to update suspended of Habit '{}', to '{}'.",
            habit_name, new_suspended
        )
    })?;

    Ok(())
}

pub fn habit_exists(conn: &Connection, habit_name: &str) -> anyhow::Result<bool> {
    match conn.query_row(
        "SELECT name FROM Habit WHERE name = ?1",
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

pub fn habit_suspended(conn: &Connection, habit_name: &str) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT suspended FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |row| row.get::<_, bool>(0),
    )
    .with_context(|| {
        format!(
            "Failed to select suspended in Habit with name '{}'.",
            habit_name
        )
    })
}

pub fn habit_get_by_name(conn: &Connection, habit_name: &str) -> anyhow::Result<Habit> {
    conn.query_row(
        "SELECT name, description, days, hour, minutes, suspended FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |row| {
            let name = row.get::<_, String>(0)?;
            let description = row.get::<_, String>(1)?;
            let days = habit::byte_to_days(row.get::<_, u8>(2)?);
            let at = At::build(row.get::<usize, u8>(3)?, row.get::<usize, u8>(4)?)
                .expect("Hour and minutes from database should be valid.");
            let suspended = row.get::<_, bool>(5)?;
            Ok(Habit::new(name, description, days, at, suspended))
        },
    )
    .with_context(|| format!("Failed to select Habit with name '{}'.", habit_name))
}

pub fn habit_get_name_with_most_recent_log(conn: &Connection) -> anyhow::Result<String> {
    let habit_id = conn
        .query_row(
            "SELECT habit_id FROM Log ORDER BY created_at DESC LIMIT 1",
            (),
            |row| row.get::<_, usize>(0),
        )
        .with_context(|| "Failed to select the name of the habit that has the most recent log.")?;

    habit_get_name_from_id(conn, habit_id)
}

pub fn habit_get_all(conn: &Connection) -> anyhow::Result<Vec<Habit>> {
    let mut stmt = conn
        .prepare("SELECT name, description, days, hour, minutes, suspended FROM Habit")
        .with_context(|| "Failed to prepare 'select all habits' statement.")?;

    let rows = stmt
        .query_map([], |row| {
            let name = row.get::<_, String>(0)?;
            let description = row.get::<_, String>(1)?;
            let days = habit::byte_to_days(row.get::<_, u8>(2)?);
            let at = At::build(row.get::<_, u8>(3)?, row.get::<_, u8>(4)?)
                .expect("Hour and minutes from database should be valid.");
            let suspended = row.get::<_, bool>(5)?;
            Ok(Habit::new(name, description, days, at, suspended))
        })
        .with_context(|| "Failed to select all habits.")?;

    let mut habits = Vec::new();
    for row in rows {
        habits.push(row?);
    }

    Ok(habits)
}

pub fn habit_get_names(conn: &Connection) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT name FROM Habit")
        .with_context(|| "Failed to prepare statement to select Habit names.")?;
    let name_results = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut names = vec![];
    for name_res in name_results {
        names.push(name_res?);
    }

    Ok(names)
}

fn habit_get_id_from_name(conn: &Connection, habit_name: &str) -> anyhow::Result<usize> {
    conn.query_row(
        "SELECT id FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |row| row.get::<_, usize>(0),
    )
    .with_context(|| format!("Failed to select id of Habit with name '{}'.", habit_name))
}

fn habit_get_name_from_id(conn: &Connection, habit_id: usize) -> anyhow::Result<String> {
    conn.query_row(
        "SELECT name FROM Habit WHERE id = ?1",
        rusqlite::params![habit_id],
        |row| row.get::<_, String>(0),
    )
    .with_context(|| format!("Failed to select name of Habit with id '{}'.", habit_id))
}

pub fn habit_delete(conn: &Connection, habit_name: &str) -> anyhow::Result<()> {
    // In sqlite, need to enable foreign keys at runtime using a pragma.
    // See <https://www.sqlite.org/foreignkeys.html>.
    // In this case, this is for the deletion to cascade to Log and HabitHistory.
    conn.execute("PRAGMA foreign_keys = ON;", ())?;
    conn.execute(
        "DELETE FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
    )
    .with_context(|| format!("Failed to delete Habit with name '{}'.", habit_name))?;

    Ok(())
}

pub fn log_insert(conn: &Connection, habit_name: &str) -> anyhow::Result<()> {
    let habit_id = habit_get_id_from_name(conn, habit_name)?;
    conn.execute(
        "INSERT INTO Log (created_at, habit_id) VALUES (?1, ?2)",
        rusqlite::params![chrono::Local::now().timestamp(), habit_id],
    )
    .with_context(|| "Failed to insert log into database.")?;

    Ok(())
}

pub fn get_n_logs_for_habit(conn: &Connection, habit_name: &str) -> anyhow::Result<usize> {
    let habit_id = habit_get_id_from_name(conn, habit_name)?;
    conn.query_row(
        "SELECT COUNT(1) FROM Log WHERE habit_id = ?1",
        rusqlite::params![habit_id],
        |row| row.get::<_, usize>(0),
    )
    .with_context(|| {
        format!(
            "Failed to count number of logged reps for Habit '{}'.",
            habit_name
        )
    })
}
