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
    fn run(&mut self) -> eyre::Result<()> {
        let conn = db::get_conn!();

        if self.verbose {
            let mut habits = db::habit_get_all(&conn)?;
            habits.sort_by_key(|habit| habit.name.clone());

            let max_width = termsize::get()
                .map(|size| size.cols)
                .unwrap_or(u16::MAX)
                .checked_sub(8)
                .unwrap_or(u16::MAX) as usize;

            for habit in habits {
                let mut header = format!("{}", habit.name.bold());
                if habit.suspended {
                    header.push_str(&format!(" {}", "(suspended)".italic()));
                }
                println!("{}", header);
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
                for line in textwrap::wrap(&format!("Created at {}.", habit.created_at), max_width)
                {
                    println!("    {}", line.italic());
                }
            }
        } else {
            let mut habit_names = db::habit_get_names(&conn)?;
            habit_names.sort();
            for habit_name in habit_names {
                println!("{}", habit_name);
            }
        }

        Ok(())
    }
}
