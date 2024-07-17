use crate::db;
use crate::habit::At;
use crate::utils;
use anyhow::Context;
use colored::Colorize;
use std::str::FromStr;

use crate::engine::Engine;
use crate::habit::{Day, Habit};
use crate::list::cli::ListCli;

pub fn get_engine(cli: ListCli) -> Box<dyn Engine> {
    Box::new(ListEngine {
        verbose: cli.verbose,
    })
}

struct ListEngine {
    verbose: bool,
}

impl Engine for ListEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        let conn = db::open_db()?;

        if self.verbose {
            let mut stmt = conn
                .prepare("SELECT name, description, days, hour, minutes FROM habit")
                .with_context(|| "Failed to select habits from database.")?;

            let habits = stmt.query_map([], |row| {
                Ok(Habit::new(
                    row.get::<usize, String>(0)?,
                    row.get::<usize, String>(1)?,
                    row.get::<usize, String>(2)?
                        .split(' ')
                        .map(|d_str| {
                            Day::from_str(d_str).expect("There is a wrong day in database.")
                        })
                        .collect(),
                    At::build(row.get::<usize, u8>(3)?, row.get::<usize, u8>(4)?)
                        .expect("There is a wrong hour or wrong minutes in database."),
                ))
            })?;

            let max_width = termsize::get()
                .map(|size| size.cols)
                .unwrap_or(u16::MAX)
                .checked_sub(8)
                .unwrap_or(u16::MAX) as usize;

            for habit in habits {
                let habit = habit?;

                println!("{}", habit.name.bold());
                for line in textwrap::wrap(&habit.description, max_width) {
                    println!("    {}", line);
                }
                for line in textwrap::wrap(
                    &format!(
                        "{} Each {} at {}.",
                        ">".bright_black(),
                        utils::display_days(&habit.days),
                        habit.at
                    ),
                    max_width,
                ) {
                    println!("    {}", line);
                }
            }
        } else {
            let mut stmt = conn
                .prepare("SELECT name FROM habit")
                .with_context(|| "Failed to select habit names from database.")?;

            let names = stmt.query_map([], |row| row.get::<usize, String>(0))?;

            for name in names {
                let name = name?;
                println!("{}", name);
            }
        }

        Ok(())
    }
}
