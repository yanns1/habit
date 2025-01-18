use crate::db;
use crate::delete::cli::DeleteCli;
use crate::engine::Engine;
use crate::prompt;
use anyhow::anyhow;

pub fn get_engine(cli: DeleteCli) -> Box<dyn Engine> {
    Box::new(DeleteEngine { habit: cli.habit })
}

struct DeleteEngine {
    habit: String,
}

impl Engine for DeleteEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        let conn = db::open_db()?;

        // check if habit exists in db, if not error
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(anyhow!("Habit '{}' does not exists!", self.habit));
        }

        // ask for confirmation
        let confirmed = prompt::ask_for_confirmation(
            &format!("Are you sure? All data for '{}' will be lost. Consider exporting it before with 'habit export {}'.",
                self.habit, self.habit)
        )?;

        // delete habit
        if confirmed {
            db::habit_delete(&conn, &self.habit)?;
            println!("Habit '{}' successfully deleted!", self.habit);
        } else {
            println!("Nothing done.");
        }

        Ok(())
    }
}
