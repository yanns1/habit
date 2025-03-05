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
        let conn = db::get_conn!();

        // Check if habit exists.
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(anyhow!("Habit '{}' does not exist!", self.habit));
        }

        // Check if habit is suspended.
        if db::habit_suspended(&conn, &self.habit)? {
            println!("Nothing done, because habit '{}' is suspended. If you want, you can resume it with `habit resume {}`.", self.habit, self.habit);
            return Ok(());
        }

        // Log a rep.
        db::log_insert(&conn, &self.habit)?;

        let n_reps = db::habit_get_n_logs(&conn, &self.habit)?;
        println!("Rep successfully logged.");
        println!(
            "Good job! You are at {} for habit '{}'.",
            format!("{} {}", n_reps, if n_reps <= 1 { "rep" } else { "reps" }).bold(),
            self.habit
        );

        Ok(())
    }
}
