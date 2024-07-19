use crate::db;
use crate::engine::Engine;
use crate::log::cli::LogCli;
use anyhow::anyhow;
use colored::Colorize;

pub fn get_engine(cli: LogCli) -> Box<dyn Engine> {
    Box::new(LogEngine { habit: cli.habit })
}

struct LogEngine {
    habit: String,
}

impl Engine for LogEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        let conn = db::open_db()?;

        // check if habit exists in db, if not error
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(anyhow!("Habit '{}' does not exists!", self.habit));
        }

        // log a rep
        db::log_insert(&conn, &self.habit)?;

        // count current number of logged reps for habit
        let n_reps = db::get_n_logs_for_habit(&conn, &self.habit)?;

        println!("Rep successfully logged.");
        println!(
            "Good job! You are at {} for habit '{}'.",
            format!("{} {}", n_reps, if n_reps <= 1 { "rep" } else { "reps" }).bold(),
            self.habit
        );

        Ok(())
    }
}
