use crate::habit;
use crate::habit::At;
use crate::habit::Day;
use crate::habit::Habit;
use crate::paths::DATA_DIR_PATH;
use chrono::DateTime;
use chrono::Local;
use chrono::TimeZone;
use eyre::eyre;
use eyre::WrapErr;
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
// `f1` locks the connection, then calls a function `f2` which also tries
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
    #[cfg(debug_assertions)]
    {
        use std::env;
        use std::path::PathBuf;

        let mut recreate_db = true;
        let db_path = if let Ok(db_str) = env::var("DB") {
            let db_path = PathBuf::from(db_str.clone());

            if db_path.exists() {
                recreate_db = false;
                db_path
            } else {
                let mut db_path = DATA_DIR_PATH.clone();
                db_path.push(format!("{}.db", db_str));

                db_path
            }
        } else {
            let mut db_path = DATA_DIR_PATH.clone();
            db_path.push("default.db");

            db_path
        };

        // Remove old test database if there is one.
        if recreate_db && db_path.exists() {
            std::fs::remove_file(&*db_path).unwrap_or_else(|_| {
                panic!(
                    "Failed to remove db at path '{}'.",
                    db_path.to_string_lossy()
                )
            });
        }

        let manager = SqliteConnectionManager::file(&*db_path);
        let pool = Pool::new(manager).expect("Failed to create database connection pool.");

        if recreate_db {
            let conn = pool
                .get()
                .expect("Failed to get a database connection from the pool.");

            create_tables(&conn).expect("Failed to create database tables.");

            let stem = db_path.file_stem().unwrap();
            let stem = stem.to_str().unwrap();
            debug::fill_db(&conn, stem).unwrap();
        }

        pool
    }

    #[cfg(not(debug_assertions))]
    {
        let mut db_path = DATA_DIR_PATH.clone();
        db_path.push("habit.db");

        // Check if the database existed. If not, need to create the tables.
        let db_existed = db_path.exists();

        let manager = SqliteConnectionManager::file(&*db_path);
        let pool = Pool::new(manager).expect("Failed to create database connection pool.");

        if !db_existed {
            let conn = pool
                .get()
                .expect("Failed to get a database connection from the pool.");

            create_tables(&conn).expect("Failed to create database tables.");
        }

        pool
    }
});

pub fn create_tables(conn: &Connection) -> eyre::Result<()> {
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
    .wrap_err("Failed to create tables.")?;

    Ok(())
}

pub fn habit_table_is_empty(conn: &Connection) -> eyre::Result<bool> {
    conn.query_row(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM Habit) THEN 0 ELSE 1 END",
        [],
        |row| row.get::<_, bool>(0),
    )
    .wrap_err("Failed to check if Habit table is empty.")
}

pub fn habit_insert(conn: &Connection, habit: &Habit) -> eyre::Result<()> {
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
        .wrap_err("Query to select max id of Habit failed.")?
    };

    let byte = habit::days_to_byte(&habit.days[..]);

    // NOTE: Use timestamp in nanoseconds so as to be consistent with the rest of
    // the tables.
    //
    // However, unlike seconds, milliseconds and microseconds, the range of datetimes for which
    // we can represent nanoseconds as a i64 is limited, meaning `timestamp_nanos_opt` can fail
    // (as expressed by the returned `Option`).
    // The range is from year 1677 to 2262 approximately.
    // See <https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.timestamp_nanos_opt> for more details.
    let created_at_timestamp = habit.created_at.timestamp_nanos_opt().unwrap();

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
    .wrap_err(format!("Failed to insert '({}, {}, {}, {}, {}, {}, {}, {})' into Habit.", id,
        habit.name, habit.description, byte, habit.at.hour, habit.at.minutes, habit.suspended, created_at_timestamp))?;

    Ok(())
}

fn habit_write_history(conn: &Connection, habit_name: &str) -> eyre::Result<()> {
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
        .wrap_err(format!("Failed to select Habit with name '{}'.", habit_name))?;

    // NOTE: Use timestamp in nanoseconds so as to minimize the likelihood that two (or more)
    // calls to this function, for a same habit, run fast enough that the timestamps are equal.
    // Because when `habit_id` and `recorded_at` are equal, the primary key constraint is
    // violated.
    //
    // However, unlike seconds, milliseconds and microseconds, the range of datetimes for which
    // we can represent nanoseconds as a i64 is limited, meaning `timestamp_nanos_opt` can fail
    // (as expressed by the returned `Option`).
    // The range is from year 1677 to 2262 approximately.
    // See <https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.timestamp_nanos_opt> for more details.
    let recorded_at_timestamp = chrono::Local::now().timestamp_nanos_opt().unwrap();
    conn.execute(
        "INSERT INTO HabitHistory (habit_id, name, description, days, hour, minutes, suspended, created_at, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![id, name, description, days, hour, minutes, suspended, created_at_timestamp, recorded_at_timestamp]
    ).wrap_err(
        format!("Failed to insert '({}, {}, {}, {}, {}, {}, {}, {}, {})' into HabitHistory.",
            id, name, description, days, hour, minutes, suspended, created_at_timestamp, recorded_at_timestamp)
    )?;

    Ok(())
}

pub fn habit_update_name(conn: &Connection, habit_name: &str, new_name: &str) -> eyre::Result<()> {
    habit_write_history(conn, habit_name)?;

    conn.execute(
        "UPDATE Habit SET name = ?1 WHERE name = ?2",
        rusqlite::params![new_name, habit_name],
    )
    .wrap_err(format!(
        "Failed to update name of habit '{}' to '{}'.",
        habit_name, new_name
    ))?;

    Ok(())
}

pub fn habit_update_description(
    conn: &Connection,
    habit_name: &str,
    new_description: &str,
) -> eyre::Result<()> {
    habit_write_history(conn, habit_name)?;

    conn.execute(
        "UPDATE Habit SET description = ?1 WHERE name = ?2",
        rusqlite::params![new_description, habit_name],
    )
    .wrap_err(format!(
        "Failed to update description of Habit '{}', to '{}'.",
        habit_name, new_description
    ))?;

    Ok(())
}

pub fn habit_update_days(
    conn: &Connection,
    habit_name: &str,
    new_days: &[Day],
) -> eyre::Result<()> {
    habit_write_history(conn, habit_name)?;

    let byte = habit::days_to_byte(new_days);
    conn.execute(
        "UPDATE Habit SET days = ?1 WHERE name = ?2",
        rusqlite::params![byte, habit_name],
    )
    .wrap_err(format!(
        "Failed to update days of Habit '{}', to '{}'.",
        habit_name, byte
    ))?;

    Ok(())
}

pub fn habit_update_at(conn: &Connection, habit_name: &str, new_at: &At) -> eyre::Result<()> {
    habit_write_history(conn, habit_name)?;

    conn.execute(
        "UPDATE Habit SET hour = ?1, minutes = ?2 WHERE name = ?3",
        rusqlite::params![new_at.hour, new_at.minutes, habit_name],
    )
    .wrap_err(format!(
        "Failed to update at of Habit '{}', to '{}'.",
        habit_name, new_at
    ))?;

    Ok(())
}

pub fn habit_update_suspended(
    conn: &Connection,
    habit_name: &str,
    new_suspended: bool,
) -> eyre::Result<()> {
    habit_write_history(conn, habit_name)?;

    conn.execute(
        "UPDATE Habit SET suspended = ?1 WHERE name = ?2",
        rusqlite::params![new_suspended, habit_name],
    )
    .wrap_err(format!(
        "Failed to update suspended of Habit '{}', to '{}'.",
        habit_name, new_suspended
    ))?;

    Ok(())
}

pub fn habit_exists(conn: &Connection, habit_name: &str) -> eyre::Result<bool> {
    match conn.query_row(
        "SELECT name FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |_| Ok(()),
    ) {
        Ok(_) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(e) => Err(eyre!(
            "Query to select habit with name '{}' failed.\n{}",
            habit_name,
            e
        )),
    }
}

pub fn habit_is_suspended(conn: &Connection, habit_name: &str) -> eyre::Result<bool> {
    conn.query_row(
        "SELECT suspended FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |row| row.get::<_, bool>(0),
    )
    .wrap_err(format!(
        "Failed to select suspended in Habit with name '{}'.",
        habit_name
    ))
}

pub fn habit_get_name_with_most_recent_log(conn: &Connection) -> eyre::Result<String> {
    let habit_id = conn
        .query_row(
            "SELECT habit_id FROM Log ORDER BY created_at DESC LIMIT 1",
            (),
            |row| row.get::<_, usize>(0),
        )
        .wrap_err("Failed to select the name of the habit that has the most recent log.")?;

    habit_get_name_from_id(conn, habit_id)
}

pub fn habit_get_all(conn: &Connection) -> eyre::Result<Vec<Habit>> {
    let mut stmt = conn
        .prepare("SELECT name, description, days, hour, minutes, suspended, created_at FROM Habit")
        .wrap_err("Failed to prepare 'select all habits' statement.")?;

    let rows = stmt
        .query_map([], |row| {
            let name = row.get::<_, String>(0)?;
            let description = row.get::<_, String>(1)?;
            let days = habit::byte_to_days(row.get::<_, u8>(2)?);
            let at = At::build(row.get::<_, u8>(3)?, row.get::<_, u8>(4)?)
                .expect("Hour and minutes from database should be valid.");
            let suspended = row.get::<_, bool>(5)?;
            let created_at_timestamp = row.get::<_, i64>(6)?;
            let created_at = Local.timestamp_nanos(created_at_timestamp);
            Ok(Habit::new(
                name,
                description,
                days,
                at,
                suspended,
                Some(created_at),
            ))
        })
        .wrap_err("Failed to select all habits.")?;

    let mut habits = Vec::new();
    for row in rows {
        habits.push(row?);
    }

    Ok(habits)
}

pub fn habit_get_names(conn: &Connection) -> eyre::Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT name FROM Habit")
        .wrap_err("Failed to prepare statement to select Habit names.")?;
    let name_results = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut names = vec![];
    for name_res in name_results {
        names.push(name_res?);
    }

    Ok(names)
}

pub fn habit_get_from_name(conn: &Connection, habit_name: &str) -> eyre::Result<Habit> {
    conn.query_row(
        "SELECT description, days, hour, minutes, suspended, created_at FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |row| {
            let description = row.get::<_, String>(0)?;
            let days = habit::byte_to_days(row.get::<_, u8>(1)?);
            let at = At::build(row.get::<_, u8>(2)?, row.get::<_, u8>(3)?)
                .expect("Hour and minutes from database should be valid.");
            let suspended = row.get::<_, bool>(4)?;
            let created_at_timestamp = row.get::<_, i64>(5)?;
            let created_at = Local.timestamp_nanos(created_at_timestamp);
            Ok(Habit::new(
                habit_name.to_string(),
                description,
                days,
                at,
                suspended,
                Some(created_at),
            ))
        },
    )
    .wrap_err(format!(
        "Failed to select Habit with name '{}'.",
        habit_name
    ))
}

fn habit_get_id_from_name(conn: &Connection, habit_name: &str) -> eyre::Result<usize> {
    conn.query_row(
        "SELECT id FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
        |row| row.get::<_, usize>(0),
    )
    .wrap_err(format!(
        "Failed to select id of Habit with name '{}'.",
        habit_name
    ))
}

fn habit_get_name_from_id(conn: &Connection, habit_id: usize) -> eyre::Result<String> {
    conn.query_row(
        "SELECT name FROM Habit WHERE id = ?1",
        rusqlite::params![habit_id],
        |row| row.get::<_, String>(0),
    )
    .wrap_err(format!(
        "Failed to select name of Habit with id '{}'.",
        habit_id
    ))
}

pub fn habit_delete(conn: &Connection, habit_name: &str) -> eyre::Result<()> {
    // In sqlite, need to enable foreign keys at runtime using a pragma.
    // See <https://www.sqlite.org/foreignkeys.html>.
    // In this case, this is for the deletion to cascade to Log and HabitHistory.
    conn.execute("PRAGMA foreign_keys = ON;", ())?;
    conn.execute(
        "DELETE FROM Habit WHERE name = ?1",
        rusqlite::params![habit_name],
    )
    .wrap_err(format!(
        "Failed to delete Habit with name '{}'.",
        habit_name
    ))?;

    Ok(())
}

pub fn log_insert(
    conn: &Connection,
    habit_name: &str,
    created_at: Option<DateTime<Local>>,
) -> eyre::Result<()> {
    let habit_id = habit_get_id_from_name(conn, habit_name)?;

    // NOTE: Use timestamp in nanoseconds so as to minimize the likelihood that two (or more)
    // calls to this function run fast enough that the timestamps are equal.
    // Because when the `created_at` values are equal, the primary key constraint is
    // violated.
    //
    // However, unlike seconds, milliseconds and microseconds, the range of datetimes for which
    // we can represent nanoseconds as a i64 is limited, meaning `timestamp_nanos_opt` can fail
    // (as expressed by the returned `Option`).
    // The range is from year 1677 to 2262 approximately.
    // See <https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.timestamp_nanos_opt> for more details.
    let created_at_timestamp = created_at
        .unwrap_or(chrono::Local::now())
        .timestamp_nanos_opt()
        .unwrap();

    conn.execute(
        "INSERT INTO Log (created_at, habit_id) VALUES (?1, ?2)",
        rusqlite::params![created_at_timestamp, habit_id],
    )
    .wrap_err("Failed to insert log into database.")?;

    Ok(())
}

pub fn habit_get_n_logs(conn: &Connection, habit_name: &str) -> eyre::Result<usize> {
    let habit_id = habit_get_id_from_name(conn, habit_name)?;
    conn.query_row(
        "SELECT COUNT(1) FROM Log WHERE habit_id = ?1",
        rusqlite::params![habit_id],
        |row| row.get::<_, usize>(0),
    )
    .wrap_err(format!(
        "Failed to count number of logged reps for Habit '{}'.",
        habit_name
    ))
}

pub fn habit_get_n_logs_for_year(
    conn: &Connection,
    habit_name: &str,
    year: i32,
) -> eyre::Result<usize> {
    let habit_id = habit_get_id_from_name(conn, habit_name)?;

    let first_second_of_year = Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let last_second_of_year = Local.with_ymd_and_hms(year, 12, 31, 23, 59, 59).unwrap();

    conn.query_row(
        "SELECT COUNT(1) FROM Log WHERE habit_id = ?1 AND (created_at BETWEEN ?2 AND ?3)",
        rusqlite::params![
            habit_id,
            first_second_of_year.timestamp_nanos_opt().unwrap(),
            last_second_of_year.timestamp_nanos_opt().unwrap(),
        ],
        |row| row.get::<_, usize>(0),
    )
    .wrap_err(format!(
        "Failed to count number of logged reps for Habit '{}'.",
        habit_name
    ))
}

pub fn habit_get_logs_between(
    conn: &Connection,
    habit_name: &str,
    dt1: &DateTime<Local>,
    dt2: &DateTime<Local>,
) -> eyre::Result<Vec<DateTime<Local>>> {
    let habit_id = habit_get_id_from_name(conn, habit_name)?;

    let mut stmt = conn
        .prepare(
            "SELECT created_at FROM Log WHERE habit_id = ?1 AND (created_at BETWEEN ?2 AND ?3)",
        )
        .wrap_err("Failed to prepare select statement.")?;

    let rows = stmt
        .query_map(
            rusqlite::params![
                habit_id,
                dt1.timestamp_nanos_opt().unwrap(),
                dt2.timestamp_nanos_opt().unwrap()
            ],
            |row| {
                let timestamp = row.get::<_, i64>(0)?;
                let datetime = Local.timestamp_nanos(timestamp);
                Ok(datetime)
            },
        )
        .wrap_err("Failed to select all logs.")?;

    let mut datetimes = Vec::new();
    for row in rows {
        datetimes.push(row?);
    }

    Ok(datetimes)
}

pub fn habit_get_logs_for_year(
    conn: &Connection,
    habit_name: &str,
    year: i32,
) -> eyre::Result<Vec<DateTime<Local>>> {
    let first_second_of_year = Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let last_second_of_year = Local.with_ymd_and_hms(year, 12, 31, 23, 59, 59).unwrap();
    habit_get_logs_between(
        conn,
        habit_name,
        &first_second_of_year,
        &last_second_of_year,
    )
}

pub fn log_table_is_empty(conn: &Connection) -> eyre::Result<bool> {
    conn.query_row(
        "SELECT CASE WHEN EXISTS(SELECT 1 FROM Log) THEN 0 ELSE 1 END",
        [],
        |row| row.get::<_, bool>(0),
    )
    .wrap_err("Failed to check if Log table is empty.")
}

#[cfg(debug_assertions)]
mod debug {
    use super::habit_insert;
    use super::habit_update_suspended;
    use super::log_insert;
    use crate::habit::At;
    use crate::habit::Day;
    use crate::habit::Habit;
    use crate::TODAY;
    use chrono::DateTime;
    use chrono::Datelike;
    use chrono::Days;
    use chrono::Local;
    use chrono::TimeZone;
    use rusqlite::Connection;

    pub fn fill_db(conn: &Connection, name: &str) -> eyre::Result<()> {
        match name {
            "default" => fill_db_default(conn),
            "empty" => Ok(()),
            "new" => fill_db_new(conn),
            "edit" => fill_db_edit(conn),
            "delete" => fill_db_delete(conn),
            "list" => fill_db_list(conn),
            "log" => fill_db_log(conn),
            "suspend" => fill_db_suspend(conn),
            "resume" => fill_db_resume(conn),
            "show" => fill_db_show(conn),
            _ => panic!("Unexpected DB name; got '{}'.", name),
        }
    }

    /// Fill default test database.
    fn fill_db_default(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;
        // To fill `HabitHistory` a bit.
        habit_update_suspended(conn, &h1.name, true)?;
        habit_update_suspended(conn, &h1.name, false)?;
        // To fill `Log` a bit.
        for dt in generate_logs_all(&h1) {
            log_insert(conn, &h1.name, Some(dt))?;
        }

        let h2 = Habit::new(
            "h2".to_string(),
            "Description of habit 2.".to_string(),
            vec![Day::Tuesday, Day::Wednesday],
            At::build(9, 30).unwrap(),
            true,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 9, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h2)?;
        // To fill `HabitHistory` a bit.
        habit_update_suspended(conn, &h2.name, true)?;
        habit_update_suspended(conn, &h2.name, false)?;
        // To fill `Log` a bit.
        for dt in generate_logs_one_in_two(&h2) {
            log_insert(conn, &h2.name, Some(dt))?;
        }

        let h3 = Habit::new(
            "h3".to_string(),
            "Description of habit 3.".to_string(),
            vec![Day::Thursday, Day::Friday, Day::Sunday],
            At::build(11, 50).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 17, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h3)?;
        // To fill `HabitHistory` a bit.
        habit_update_suspended(conn, &h3.name, true)?;
        habit_update_suspended(conn, &h3.name, false)?;
        // To fill `Log` a bit.
        for dt in generate_logs_all(&h3) {
            log_insert(conn, &h3.name, Some(dt))?;
        }

        Ok(())
    }

    /// Fill database for testing subcomand `habit new`.
    ///
    /// # Manual tests
    ///
    /// - Check that the name prompt errors when "h1", "h2", "h3", "h4" on "h5" is entered,
    ///   because "habit already exists". Any other name should work.
    /// - The description prompt should accept anything.
    /// - Press spacebar to select a day, Enter to confirm the whole selection.
    ///   Any combination of selected/unselected days should work.
    /// - Check that the at prompt errors when the format of the input is not "hh:mm",
    ///   or when the input hour/minutes is out of bounds ([[0, 23]] and [[0, 59]]).
    /// - Check if the proper data was written to the database by running:
    ///     `SELECT * FROM Habit;`
    ///
    fn fill_db_new(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;

        let h2 = Habit::new(
            "h2".to_string(),
            "Description of habit 2.".to_string(),
            vec![Day::Tuesday, Day::Wednesday],
            At::build(9, 30).unwrap(),
            true,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 9, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h2)?;

        let h3 = Habit::new(
            "h3".to_string(),
            "Description of habit 3.".to_string(),
            vec![Day::Thursday, Day::Friday, Day::Sunday],
            At::build(11, 50).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 17, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h3)?;

        let h4 = Habit::new(
            "h4".to_string(),
            "Description of habit 4.".to_string(),
            vec![
                Day::Monday,
                Day::Tuesday,
                Day::Wednesday,
                Day::Thursday,
                Day::Friday,
                Day::Saturday,
                Day::Sunday,
            ],
            At::build(12, 10).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 25, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h4)?;

        let h5 = Habit::new(
            "h5".to_string(),
            "Description of habit 5.".to_string(),
            vec![Day::Monday, Day::Wednesday, Day::Friday, Day::Sunday],
            At::build(7, 15).unwrap(),
            false,
            Some(Local.with_ymd_and_hms(TODAY.year(), 2, 1, 7, 0, 0).unwrap()),
        );
        habit_insert(conn, &h5)?;

        Ok(())
    }

    /// Fill database for testing subcomand `habit edit`.
    ///
    /// # Manual tests
    ///
    /// - Run `DB=edit cargo r -- edit whatever name`.
    ///     It should fail because habit "whatever" does not exist.
    /// - Run `DB=edit cargo r -- edit h1 name`.
    ///     It should work for any name other than those already existing.
    ///     Check that the new name was written to the DB using:
    ///     `SELECT * FROM Habit;`
    ///     Check that there is a new entry in table `HabitHistory`:
    ///     `SELECT * FROM HabitHistory;`
    /// - Run `DB=edit cargo r -- edit h1 description`.
    ///     It should work for any description.
    ///     Check that the new description was written to the DB using:
    ///     `SELECT * FROM Habit;`
    ///     Check that there is a new entry in table `HabitHistory`:
    ///     `SELECT * FROM HabitHistory;`
    /// - Run `DB=edit cargo r -- edit h1 days`.
    ///     It should work for any combination of days.
    ///     Check that the new days were written to the DB using:
    ///     `SELECT * FROM Habit;`
    ///     Check that there is a new entry in table `HabitHistory`:
    ///     `SELECT * FROM HabitHistory;`
    /// - Run `DB=edit cargo r -- edit h1 at`.
    ///     The prompt should error if the format is wrong, or the hour/minutes
    ///     is out of bounds.
    ///     Check that the new at was written to the DB using:
    ///     `SELECT * FROM Habit;`
    ///     Check that there is a new entry in table `HabitHistory`:
    ///     `SELECT * FROM HabitHistory;`
    ///
    fn fill_db_edit(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;

        Ok(())
    }

    /// Fill database for testing subcomand `habit delete`.
    ///
    /// # Manual tests
    ///
    /// - Run `DB=delete cargo r -- delete whatever`.
    ///     It should fail because habit "whatever" does not exist.
    /// - Run `DB=delete cargo r -- delete h1`.
    ///     It should ask for confirmation, and do nothing if "n" is pressed.
    /// - Run `DB=delete cargo r -- delete h1`.
    ///     It should ask for confirmation, and succeed if "y" is pressed.
    ///     Check that the entry for `h1` no longer is in table `Habit`:
    ///     `SELECT * FROM Habit;`
    ///     Deletion should cascade on `Log` and `HabitHistory`.
    ///     Make sure this is the case:
    ///     `SELECT * FROM Log WHERE habit_id = 0;`
    ///     `SELECT * FROM HabitHistory; WHERE habit_id = 0;`
    fn fill_db_delete(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;
        // To fill `HabitHistory` a bit.
        habit_update_suspended(conn, &h1.name, true)?;
        habit_update_suspended(conn, &h1.name, false)?;
        // To fill `Log` a bit.
        for dt in generate_logs_all(&h1) {
            log_insert(conn, &h1.name, Some(dt))?;
        }

        let h2 = Habit::new(
            "h2".to_string(),
            "Description of habit 2.".to_string(),
            vec![Day::Tuesday, Day::Wednesday],
            At::build(9, 30).unwrap(),
            true,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 9, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h2)?;
        // To fill `HabitHistory` a bit.
        habit_update_suspended(conn, &h2.name, true)?;
        habit_update_suspended(conn, &h2.name, false)?;
        // To fill `Log` a bit.
        for dt in generate_logs_one_in_two(&h2) {
            log_insert(conn, &h2.name, Some(dt))?;
        }

        let h3 = Habit::new(
            "h3".to_string(),
            "Description of habit 3.".to_string(),
            vec![Day::Thursday, Day::Friday, Day::Sunday],
            At::build(11, 50).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 17, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h3)?;
        // To fill `HabitHistory` a bit.
        habit_update_suspended(conn, &h3.name, true)?;
        habit_update_suspended(conn, &h3.name, false)?;
        // To fill `Log` a bit.
        for dt in generate_logs_all(&h3) {
            log_insert(conn, &h3.name, Some(dt))?;
        }

        Ok(())
    }

    /// Fill database for testing subcomand `habit list`.
    ///
    /// # Manual tests
    ///
    /// - Run ``.
    /// - Run `DB=list cargo r -- list`, output should be:
    ///
    ///     ```text
    ///     h1
    ///     h2
    ///     h3
    ///     h4
    ///     h5
    ///     ```
    ///
    /// - Run `habit list -v`, check that the output corresponds
    ///     to the inserted data.
    fn fill_db_list(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;

        let h2 = Habit::new(
            "h2".to_string(),
            "Description of habit 2.".to_string(),
            vec![Day::Tuesday, Day::Wednesday],
            At::build(9, 30).unwrap(),
            true,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 9, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h2)?;

        let h3 = Habit::new(
            "h3".to_string(),
            "Description of habit 3.".to_string(),
            vec![Day::Thursday, Day::Friday, Day::Sunday],
            At::build(11, 50).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 17, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h3)?;

        let h4 = Habit::new(
            "h4".to_string(),
            "Description of habit 4.".to_string(),
            vec![
                Day::Monday,
                Day::Tuesday,
                Day::Wednesday,
                Day::Thursday,
                Day::Friday,
                Day::Saturday,
                Day::Sunday,
            ],
            At::build(12, 10).unwrap(),
            true,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 25, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h4)?;

        let h5 = Habit::new(
            "h5".to_string(),
            "Description of habit 5.".to_string(),
            vec![Day::Monday, Day::Wednesday, Day::Friday, Day::Sunday],
            At::build(7, 15).unwrap(),
            false,
            Some(Local.with_ymd_and_hms(TODAY.year(), 2, 1, 7, 0, 0).unwrap()),
        );
        habit_insert(conn, &h5)?;

        Ok(())
    }

    /// Fill database for testing subcomand `habit log`.
    ///
    /// # Manual tests
    ///
    /// - Run `DB=log cargo r -- log whatever`.
    ///     It should fail because habit "whatever" does not exist.
    /// - Run `DB=log cargo r -- log h1`.
    ///     It should succeed.
    ///     Check that the log has been added to the database:
    ///     `SELECT * FROM Log;`
    ///     This should be the only log of h1.
    /// - Run `DB=log cargo r -- log h2`.
    ///     It should succeed.
    ///     Check that the log has been added to the database:
    ///     `SELECT * FROM Log;`
    ///     This is not the only log of h2.
    /// - Run `DB=log cargo r -- log h3`.
    ///     It should succeed, but do nothing and warn that h3 is suspended.
    fn fill_db_log(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;

        let h2 = Habit::new(
            "h2".to_string(),
            "Description of habit 2.".to_string(),
            vec![Day::Tuesday, Day::Wednesday],
            At::build(9, 30).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 9, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h2)?;
        for dt in generate_logs_all(&h2) {
            log_insert(conn, &h2.name, Some(dt))?;
        }

        let h3 = Habit::new(
            "h3".to_string(),
            "Description of habit 3.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            true,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h3)?;

        Ok(())
    }

    /// Fill database for testing subcomand `habit suspend`.
    ///
    /// # Manual tests
    ///
    /// - Run `habit suspend h1`.
    ///     It should succeed.
    ///     Check in the database that the `suspended` column is set to 1 for `h1`.
    /// - Run `habit suspend whatever`.
    ///     It should fail because habit "whatever" does not exist.
    fn fill_db_suspend(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;

        Ok(())
    }

    /// Fill database for testing subcomand `habit resume`.
    ///
    /// # Manual tests
    ///
    /// - Run `DB=resume cargo r -- resume h1`.
    ///     It should succeed.
    ///     Check in the database that the `suspended` column is set to 0 for `h1`.
    /// - Run `DB=resume cargo r -- resume h2`.
    ///     It should succeed, but do nothing, and provide a warning saying
    ///     that h2 is already resumed.
    /// - Run `DB=resume cargo r -- resume whatever`.
    ///     It should fail because habit "whatever" does not exist.
    fn fill_db_resume(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            true,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;

        let h2 = Habit::new(
            "h2".to_string(),
            "Description of habit 2.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h2)?;

        Ok(())
    }

    /// Fill database for testing subcomand `habit show`.
    ///
    /// # Manual tests
    ///
    /// TODO
    fn fill_db_show(conn: &Connection) -> eyre::Result<()> {
        let h1 = Habit::new(
            "h1".to_string(),
            "Description of habit 1.".to_string(),
            vec![Day::Monday],
            At::build(8, 0).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h1)?;
        // To fill `Log` a bit.
        for dt in generate_logs_all(&h1) {
            log_insert(conn, &h1.name, Some(dt))?;
        }

        let h2 = Habit::new(
            "h2".to_string(),
            "Description of habit 2.".to_string(),
            vec![Day::Tuesday, Day::Wednesday],
            At::build(9, 30).unwrap(),
            true,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 9, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h2)?;
        // To fill `Log` a bit.
        for dt in generate_logs_one_in_two(&h2) {
            log_insert(conn, &h2.name, Some(dt))?;
        }

        let h3 = Habit::new(
            "h3".to_string(),
            "Description of habit 3.".to_string(),
            vec![Day::Thursday, Day::Friday, Day::Sunday],
            At::build(11, 50).unwrap(),
            false,
            Some(
                Local
                    .with_ymd_and_hms(TODAY.year(), 1, 17, 10, 0, 0)
                    .unwrap(),
            ),
        );
        habit_insert(conn, &h3)?;
        // To fill `Log` a bit.
        for dt in generate_logs_all(&h3) {
            log_insert(conn, &h3.name, Some(dt))?;
        }

        Ok(())
    }

    fn generate_logs_all(habit: &Habit) -> Vec<DateTime<Local>> {
        let mut dts: Vec<DateTime<Local>> = vec![];
        let mut dt = habit.created_at;
        let one_day = Days::new(1);
        while dt <= *TODAY {
            if habit.days.contains(&dt.weekday().into()) {
                dts.push(dt);
            }

            dt = dt.checked_add_days(one_day).unwrap_or_else(|| {
                panic!(
                    "Failed to add one day to '{}'. Probably a daylight saving time transition.",
                    dt
                )
            });
        }

        dts
    }

    fn generate_logs_one_in_two(habit: &Habit) -> Vec<DateTime<Local>> {
        let mut dts: Vec<DateTime<Local>> = vec![];
        let mut dt = habit.created_at;
        let mut should_add = true;
        let one_day = Days::new(1);
        while dt <= *TODAY {
            if habit.days.contains(&dt.weekday().into()) {
                if should_add {
                    dts.push(dt);
                }
                should_add = !should_add;
            }

            dt = dt.checked_add_days(one_day).unwrap_or_else(|| {
                panic!(
                    "Failed to add one day to '{}'. Probably a daylight saving time transition.",
                    dt
                )
            });
        }

        dts
    }
}
