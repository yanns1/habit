use crate::db;
use crate::engine::Engine;
use crate::habit::Day;
use crate::log::cli::LogCli;
use crate::log::cli::PastDate;
use crate::macros::get_conn;
use crate::utils;
use crate::TODAY;
use chrono::Datelike;
use chrono::Days;
use chrono::Local;
use chrono::TimeZone;
use chrono::Weekday;
use colored::Colorize;
use eyre::eyre;

pub fn get_engine(cli: LogCli) -> Box<dyn Engine> {
    Box::new(LogEngine {
        habit: cli.habit,
        date: cli.date,
    })
}

struct LogEngine {
    habit: String,
    date: Option<PastDate>,
}

impl Engine for LogEngine {
    fn run(&mut self) -> eyre::Result<()> {
        let conn = get_conn!();

        // Check if habit exists.
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(eyre!("Habit '{}' does not exist!", self.habit));
        }

        let habit = db::habit_get_from_name(&conn, &self.habit)?;

        // Check if habit is suspended.
        if habit.suspended {
            println!("Nothing done, because habit '{}' is suspended. If you want, you can resume it with `habit resume {}`.",
                habit.name, habit.name);
            return Ok(());
        }

        match &self.date {
            Some(past_date) => match past_date {
                PastDate::Last => {
                    // Find last habit day.
                    let one_day = Days::new(1);
                    let mut last_habit_day_dt = chrono::Local::now();
                    debug_assert!(!habit.days.is_empty());
                    while !habit.days.contains(&last_habit_day_dt.weekday().into()) {
                        last_habit_day_dt = last_habit_day_dt.checked_sub_days(one_day).ok_or(eyre::eyre!(
                            "Failed to subtract one day to '{}'. Probably a daylight saving time transition.",
                            last_habit_day_dt
                        ))?;
                    }

                    // Check if last habit day has already been logged.
                    let first_second_of_last_habit_day = Local
                        .with_ymd_and_hms(
                            last_habit_day_dt.year(),
                            last_habit_day_dt.month(),
                            last_habit_day_dt.day(),
                            0,
                            0,
                            0,
                        )
                        .unwrap();
                    let last_second_of_last_habit_day = Local
                        .with_ymd_and_hms(
                            last_habit_day_dt.year(),
                            last_habit_day_dt.month(),
                            last_habit_day_dt.day(),
                            23,
                            59,
                            59,
                        )
                        .unwrap();
                    let last_habit_day_logs = db::habit_get_logs_between(
                        &conn,
                        &habit.name,
                        &first_second_of_last_habit_day,
                        &last_second_of_last_habit_day,
                    )?;
                    debug_assert!(last_habit_day_logs.len() <= 1);
                    if !last_habit_day_logs.is_empty() {
                        eprintln!(
                            "Habit '{}' has already been logged for last habit day ({}). Nothing done.",
                            habit.name, <Weekday as Into<Day>>::into(last_habit_day_dt.weekday())
                        );
                        return Ok(());
                    }

                    // Log the rep.
                    db::log_insert(&conn, &habit.name, Some(last_habit_day_dt))?;
                    println!(
                        "Rep successfully logged for last habit day ({}).",
                        last_habit_day_dt
                    );

                    let n_reps = db::habit_get_n_logs(&conn, &habit.name)?;
                    println!(
                        "Good job! You are at {} for habit '{}'.",
                        format!("{} {}", n_reps, if n_reps <= 1 { "rep" } else { "reps" }).bold(),
                        habit.name
                    );
                }
            },
            None => {
                // Check if today is a habit day.
                if !habit.days.contains(&TODAY.weekday().into()) {
                    eprintln!(
                        "Today is not a habit day of '{}' ({}), so you cannot log a rep.",
                        habit.name,
                        utils::display_days(&habit.days)
                    );
                    return Ok(());
                }

                // Check if today has already been logged.
                let first_second_of_today = Local
                    .with_ymd_and_hms(TODAY.year(), TODAY.month(), TODAY.day(), 0, 0, 0)
                    .unwrap();
                let last_second_of_today = Local
                    .with_ymd_and_hms(TODAY.year(), TODAY.month(), TODAY.day(), 23, 59, 59)
                    .unwrap();
                let today_logs = db::habit_get_logs_between(
                    &conn,
                    &habit.name,
                    &first_second_of_today,
                    &last_second_of_today,
                )?;
                debug_assert!(today_logs.len() <= 1);
                if !today_logs.is_empty() {
                    eprintln!(
                        "Habit '{}' has already been logged today. Nothing done.",
                        habit.name
                    );
                    return Ok(());
                }

                // Log the rep.
                let now = Local::now();
                db::log_insert(&conn, &habit.name, Some(now))?;
                println!("Rep successfully logged ({}).", now);

                let n_reps = db::habit_get_n_logs(&conn, &habit.name)?;
                println!(
                    "Good job! You are at {} for habit '{}'.",
                    format!("{} {}", n_reps, if n_reps <= 1 { "rep" } else { "reps" }).bold(),
                    habit.name
                );
            }
        }
        Ok(())
    }
}
