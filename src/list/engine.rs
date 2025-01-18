use crate::db;
use crate::engine::Engine;
use crate::list::cli::ListCli;
use crate::utils;
use colored::Colorize;

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
            let habits = db::habit_get_all(&conn)?;

            let max_width = termsize::get()
                .map(|size| size.cols)
                .unwrap_or(u16::MAX)
                .checked_sub(8)
                .unwrap_or(u16::MAX) as usize;

            for habit in habits {
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
            let habit_names = db::habit_get_names(&conn)?;
            for habit_name in habit_names {
                println!("{}", habit_name);
            }
        }

        Ok(())
    }
}
