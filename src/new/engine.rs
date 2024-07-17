use crate::db;
use crate::prompt;

use crate::engine::Engine;
use crate::habit::Habit;
use crate::new::cli::NewCli;

pub fn get_engine(cli: NewCli) -> Box<dyn Engine> {
    let _ = cli;
    Box::new(NewEngine {})
}

struct NewEngine {}

impl Engine for NewEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        // ask habit info
        let name = prompt::prompt_habit_name()?;
        let description = prompt::prompt_habit_description()?;
        let days = prompt::prompt_habit_days()?;
        let at = prompt::prompt_habit_at()?;
        let habit = Habit::new(name, description, days, at);

        // add to DB
        let conn = db::open_db()?;
        db::habit_insert(&conn, &habit)?;

        println!("Habit '{}' successfully created!", habit.name);
        println!("Run 'habit log {}' to log progress.", habit.name);
        println!("Run 'habit show {}' to show progress.", habit.name);

        Ok(())
    }
}
