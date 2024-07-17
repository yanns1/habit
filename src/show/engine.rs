use super::{viz::BowlOfMarbles, viz::HeatMap, viz::ProgressVisualizer, How};
use crate::db;
use crate::engine::Engine;
use crate::show::cli::ShowCli;
use anyhow::{anyhow, Context};
use colored::Colorize;

pub fn get_engine(cli: ShowCli) -> Box<dyn Engine> {
    Box::new(ShowEngine {
        habit: cli.habit,
        how: cli.how,
    })
}

struct ShowEngine {
    habit: String,
    how: How,
}

impl Engine for ShowEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        let conn = db::open_db()?;

        // check if habit exists in db, if not error
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(anyhow!("Habit '{}' does not exists!", self.habit));
        }

        // Show current number of logged reps
        let n_reps = db::get_n_logs_for_habit(&conn, &self.habit)?;
        println!(
            "You accumulated {} for habit '{}'. Congratulations!",
            format!("{} {}", n_reps, if n_reps <= 1 { "rep" } else { "reps" }).bold(),
            self.habit
        );

        println!();
        match self.how {
            How::HeatMap => {
                HeatMap::new().show_progress(&self.habit).with_context(|| {
                    format!("Failed to build heatmap for habit '{}'.", self.habit)
                })?;
            }
            How::BowlOfMarbles => {
                BowlOfMarbles::new().show_progress(&self.habit)?;
            }
        }

        Ok(())
    }
}
