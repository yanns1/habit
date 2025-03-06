use crate::habit;
use crate::habit::At;
use crate::habit::Day;
use crate::habit::Habit;
use crate::paths::DB_PATH;
use anyhow::anyhow;
use anyhow::Context;
use chrono::DateTime;
use chrono::Local;
use chrono::TimeZone;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::sync::LazyLock;

// # About database connection management
//
// The naive strategy is to create a connection each time one is needed.
// However, it has two important downsides:
//
// 1. Connecting to the database can be costly, especially
//    if communication with the database is done through the network.
// 2. Creating connections in many different places clutters the code,
//    and introduces that many failure points.
//
// Thus came to me the idea of making one global connection for the
// entire app. In principle, there are multiple ways to make such global:
//
// 1. Use the `const` keyword in conjunction with `std::cell::LazyCell`.
// 2. Use the `static` keyword in conjunction with `std::sync::LazyLock`.
//
// 1 compiles, but triggers a warning from clippy: a `const` item should
// not be interior mutable. Indeed, the connection type of rusqlite,
// `rusqlite::Connection`, makes use of the interior mutability pattern.
// The documentation for this lint is as follows:
//
// > Consts are copied everywhere they are referenced, i.e., every time
// > you refer to the const a fresh instance of the Cell or Mutex or
// > AtomicXxxx will be created, which defeats the whole purpose of using
// > these types in the first place.
// >
// > The const should better be replaced by a `static` item if a global
// > variable is wanted, or replaced by a `const fn` if a constructor is wanted.
//
// So 1 does not work because `rusqlite::Connection` is interior mutable,
// and `const` amounts to copying the `LazyCell` everywhere it is referenced,
// which comes down to having one connection per reference.
//
// The lint suggests using `static`, which is point 2. We cannot use
// `LazyCell` with `static`, because `static`s are required to by `Sync`.
// The [documentation](https://doc.rust-lang.org/std/keyword.static.html)
// explains that `static`s are required to be `Sync`, because they are
// expected to always be safe to read. They are unsafe to write, however,
// an unsafe block is required to do so. See the documentation of `static`
// and `const` to see the differences, as well as
// <https://doc.rust-lang.org/reference/items/static-items.html#r-items.static.alternate>.
//
// We can use the thread-safe version of `LazyCell`: `std::sync::LazyLock`.
// However, it still does not work, because, `LazyLock` requires it containing
// value to be `Sync` as well, which `rusqlite::Connection` is not.
// `rusqlite::Connection` is `Send`, however. There has been discussion on
// why `rusqlite::Connection` is `Send` but not `Sync`:
//
// - <https://github.com/rusqlite/rusqlite/issues/188#issuecomment-787390455>
// - <https://github.com/rusqlite/rusqlite/issues/342#issuecomment-592662942>
// - <https://github.com/rusqlite/rusqlite/discussions/1226>
//
// The lint also suggests using `const fn`, which are functions that are
// permitted to be called in place of a `const` expression. When used
// in a `const` context, the function is executed at compile time, in
// the host (and not user) environment (see
// <https://doc.rust-lang.org/reference/const_eval.html#const-functions>).
// The creation of the database connection cannot happen in a `const fn`,
// because `Connection::open` is not `const`, the path of the database
// file is determined at runtime, etc.
//
// A last idea is to use `std::sync::Mutex`, which would work,
// in a `LazyLock`, as a `static`. However, the code would still
// be cluttered with calls to `Mutex::lock`, which can fail, also.
//
// `LazyCell` and `LazyLock` strategies suffer a problem I did not
// foresee, which is that some use of the connection require a
// _mutable_ reference to it. This is the case of `Connection::transaction`
// for example. So `LazyCell`/`LazyLock` would have failed, because they
// only provide shared references. `Mutex` provides both kinds of references.
//
// Apart from the strangeness of using `Mutex` in our single-threaded situation,
// it has a limitation in multi-threaded scenarios, which is that it would
// block threads when trying to lock the connection, if it is already locked.
// This basically means we would not take advantage of the concurrency of
// the database. But this application is single-threaded, so that should
// work, right!? Not always, there can be a kind of deadlock. If a function
// `f1 locks the connection, then calls a function `f2` which also tries
// to lock the connection, `f2` will hang forever, as showcased
// [here](https://users.rust-lang.org/t/how-to-use-mutex-correctly-between-functions/55071).
// This is because `Mutex` is unlocked by its destructor, at the end of the scope.
// This can easily be worked around by either unlocking the mutex manually (`Mutex::drop`),
// by putting the code in a block which ends before the call to `f2`, or by using a
// ["raw mutex"](https://docs.rs/parking_lot/latest/parking_lot/struct.RawMutex.html).
//
// Lastly, a connection pool can be used. It is the most powerful solution, as it:
//
// - is rather easy to understand,
// - does not clutter the code too much,
// - works in single-threaded and multi-threaded scenarios,
// - is good for performance.
//
// A popular crate that implements a connection pool in Rust for sqlite is
// [r2d2_sqlite](https://docs.rs/r2d2_sqlite/latest/r2d2_sqlite/).

/// The database connection pool.
///
/// It is lazily initialized.
///
/// If the database does not exist, it is created and filled with the tables
/// as part of the initialization.
pub static DB_CONN_POOL: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    // Check if the database existed. If not, need to create the tables.
    let db_existed = DB_PATH.exists();

    let manager = SqliteConnectionManager::file(&*DB_PATH);
    let pool = Pool::new(manager).expect("Failed to create database connection pool.");

    if !db_existed {
        let conn = pool
            .get()
            .expect("Failed to get a database connection from the pool.");
        create_tables(&conn).expect("Failed to create database tables.");
    }

    pool
});

/// Macro to get a database connection from the connection pool.
macro_rules! get_conn {
    () => {{
        $crate::db::DB_CONN_POOL
            .get()
            .expect("Failed to get a database connection from the pool.")
    }};
}
// NOTE: The line below is to make the macro public to this crate only.
// Explanations here <https://stackoverflow.com/a/31749071>.
// I am still do not quite understand why `pub(crate)` instead of `pub`,
// however.
pub(crate) use get_conn;

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
            created_at  INTEGER NOT NULL,
            PRIMARY KEY (name)
        );
        CREATE TABLE HabitHistory (
            habit_id    INTEGER NOT NULL,
            name        TEXT NOT NULL,
            description TEXT NOT NULL,
            days        INTEGER NOT NULL,
            hour        INTEGER NOT NULL,
            minutes     INTEGER NOT NULL,
            suspended   INTEGER NOT NULL,
            created_at  INTEGER NOT NULL,
            recorded_at INTEGER NOT NULL,
            PRIMARY KEY (habit_id, recorded_at),
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

pub fn habit_table_is_empty(conn: &Connection) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM Habit) THEN 0 ELSE 1 END",
        [],
        |row| row.get::<_, bool>(0),
    )
    .with_context(|| "Failed to check if Habit table is empty.")
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
    let id = if habit_table_is_empty(conn)? {
        0
    } else {
        conn.query_row("SELECT MAX(id) FROM Habit", [], |row| {
            row.get::<_, usize>(0).map(|id| id + 1)
        })
        .with_context(|| "Query to select max id of Habit failed.")?
    };

    let byte = habit::days_to_byte(&habit.days[..]);

    let created_at_timestamp = habit.created_at.timestamp();

    conn.execute(
        "INSERT INTO Habit (id, name, description, days, hour, minutes, suspended, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            id,
            habit.name,
            habit.description,
            byte,
            habit.at.hour,
            habit.at.minutes,
            habit.suspended,
            created_at_timestamp,
        ],
    )
    .with_context(|| format!("Failed to insert '({}, {}, {}, {}, {}, {}, {}, {})' into Habit.", id,
        habit.name, habit.description, byte, habit.at.hour, habit.at.minutes, habit.suspended, created_at_timestamp))?;

    Ok(())
}

fn habit_write_history(conn: &Connection, habit_name: &str) -> anyhow::Result<()> {
    let (id, name, description, days, hour, minutes, suspended, created_at_timestamp) = conn
        .query_row(
            "SELECT id, name, description, days, hour, minutes, suspended, created_at FROM Habit WHERE name = ?1",
            rusqlite::params![habit_name],
            |row| {
                let id = row.get::<_, usize>(0)?;
                let name = row.get::<_, String>(1)?;
                let description = row.get::<_, String>(2)?;
                let days = row.get::<_, u8>(3)?;
                let hour = row.get::<_, u8>(4)?;
                let minutes = row.get::<_, u8>(5)?;
                let suspended = row.get::<_, bool>(6)?;
                let created_at_timestamp = row.get::<_, i64>(7)?;
                Ok((id, name, description, days, hour, minutes, suspended, created_at_timestamp))
            },
        )
        .with_context(|| format!("Failed to select Habit with name '{}'.", habit_name))?;

    let recorded_at_timestamp = chrono::Local::now().timestamp();
    conn.execute(
        "INSERT INTO HabitHistory (habit_id, name, description, days, hour, minutes, suspended, created_at, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![id, name, description, days, hour, minutes, suspended, created_at_timestamp, recorded_at_timestamp]
    ).with_context(|| {
        format!("Failed to insert '({}, {}, {}, {}, {}, {}, {}, {}, {})' into HabitHistory.",
            id, name, description, days, hour, minutes, suspended, created_at_timestamp, recorded_at_timestamp)
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

pub fn habit_is_suspended(conn: &Connection, habit_name: &str) -> anyhow::Result<bool> {
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
        "SELECT name, description, days, hour, minutes, suspended, created_at FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |row| {
            let name = row.get::<_, String>(0)?;
            let description = row.get::<_, String>(1)?;
            let days = habit::byte_to_days(row.get::<_, u8>(2)?);
            let at = At::build(row.get::<usize, u8>(3)?, row.get::<usize, u8>(4)?)
                .expect("Hour and minutes from database should be valid.");
            let suspended = row.get::<_, bool>(5)?;
            let created_at_timestamp = row.get::<_, i64>(6)?;
            let created_at = DateTime::from_timestamp(created_at_timestamp, 0)
                .expect("Timestamp stored in database should be that returned by DateTime::timestamp, unchanged.")
                .with_timezone(&Local);
            Ok(Habit::new(name, description, days, at, suspended, Some(created_at)))
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
        .prepare("SELECT name, description, days, hour, minutes, suspended, created_at FROM Habit")
        .with_context(|| "Failed to prepare 'select all habits' statement.")?;

    let rows = stmt
        .query_map([], |row| {
            let name = row.get::<_, String>(0)?;
            let description = row.get::<_, String>(1)?;
            let days = habit::byte_to_days(row.get::<_, u8>(2)?);
            let at = At::build(row.get::<_, u8>(3)?, row.get::<_, u8>(4)?)
                .expect("Hour and minutes from database should be valid.");
            let suspended = row.get::<_, bool>(5)?;
            let created_at_timestamp = row.get::<_, i64>(6)?;
            let created_at = DateTime::from_timestamp(created_at_timestamp, 0)
                .expect("Timestamp stored in database should be that returned by DateTime::timestamp, unchanged.")
                .with_timezone(&Local);
            Ok(Habit::new(name, description, days, at, suspended, Some(created_at)))
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

pub fn habit_get_n_logs(conn: &Connection, habit_name: &str) -> anyhow::Result<usize> {
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

pub fn habit_get_n_logs_for_year(
    conn: &Connection,
    habit_name: &str,
    year: i32,
) -> anyhow::Result<usize> {
    let habit_id = habit_get_id_from_name(conn, habit_name)?;

    let first_second_of_year = Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let last_second_of_year = Local.with_ymd_and_hms(year, 12, 31, 23, 59, 59).unwrap();

    conn.query_row(
        "SELECT COUNT(1) FROM Log WHERE habit_id = ?1 AND (created_at BETWEEN ?2 AND ?3)",
        rusqlite::params![
            habit_id,
            first_second_of_year.timestamp(),
            last_second_of_year.timestamp()
        ],
        |row| row.get::<_, usize>(0),
    )
    .with_context(|| {
        format!(
            "Failed to count number of logged reps for Habit '{}'.",
            habit_name
        )
    })
}

pub fn habit_get_logs_for_year(
    conn: &Connection,
    habit_name: &str,
    year: i32,
) -> anyhow::Result<Vec<DateTime<Local>>> {
    let habit_id = habit_get_id_from_name(conn, habit_name)?;

    let first_second_of_year = Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let last_second_of_year = Local.with_ymd_and_hms(year, 12, 31, 23, 59, 59).unwrap();

    let mut stmt = conn
        .prepare(
            "SELECT created_at FROM Log WHERE habit_id = ?1 AND (created_at BETWEEN ?2 AND ?3)",
        )
        .with_context(|| "Failed to prepare statement in `get_logs_for_habit`.")?;

    let rows = stmt
        .query_map(rusqlite::params![habit_id, first_second_of_year.timestamp(), last_second_of_year.timestamp()], |row| {
            let timestamp = row.get::<_, i64>(0)?;
            let datetime = DateTime::from_timestamp(timestamp, 0).expect("Timestamp stored in database should be that returned by DateTime::timestamp, unchanged.").with_timezone(&Local);
            Ok(datetime)
        })
        .with_context(|| "Failed to select all logs.")?;

    let mut datetimes = Vec::new();
    for row in rows {
        datetimes.push(row?);
    }

    Ok(datetimes)
}

pub fn log_table_is_empty(conn: &Connection) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM Log) THEN 0 ELSE 1 END",
        [],
        |row| row.get::<_, bool>(0),
    )
    .with_context(|| "Failed to check if Log table is empty.")
}
