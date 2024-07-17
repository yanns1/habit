use crate::edit::cli::What;
use crate::{db, prompt};
use anyhow::anyhow;

use crate::edit::cli::EditCli;
use crate::engine::Engine;

pub fn get_engine(cli: EditCli) -> Box<dyn Engine> {
    Box::new(EditEngine {
        habit: cli.habit,
        what: cli.what,
    })
}

struct EditEngine {
    habit: String,
    what: What,
}

impl Engine for EditEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        let conn = db::open_db()?;

        // check if habit exists in db, if not error
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(anyhow!("Habit '{}' does not exists!", self.habit));
        }

        // show input depending on what, then update db
        match self.what {
            What::Name => {
                let new_name = prompt::prompt_habit_name()?;
                db::habit_update_name(&conn, &self.habit, &new_name)?;
                println!("Name successfully updated!");
                Ok(())
            }
            What::Description => {
                let new_description = prompt::prompt_habit_description()?;
                db::habit_update_description(&conn, &self.habit, &new_description)?;
                println!("Description successfully updated!");
                Ok(())
            }
            What::Days => {
                let new_days = prompt::prompt_habit_days()?;
                db::habit_update_days(&conn, &self.habit, &new_days)?;
                println!("Days successfully updated!");
                Ok(())
            }
            What::At => {
                let at = prompt::prompt_habit_at()?;
                db::habit_update_at(&conn, &self.habit, &at)?;
                println!("At successfully updated!");
                Ok(())
            }
        }
    }
}
