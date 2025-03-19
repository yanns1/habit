use crate::db;
use crate::delete::cli::DeleteCli;
use crate::engine::Engine;
use crate::macros::get_conn;
use crate::prompt;
use eyre::eyre;

pub fn get_engine(cli: DeleteCli) -> Box<dyn Engine> {
    Box::new(DeleteEngine { habit: cli.habit })
}

struct DeleteEngine {
    habit: String,
}

impl Engine for DeleteEngine {
    fn run(&mut self) -> eyre::Result<()> {
        let conn = get_conn!();

        // check if habit exists in db, if not error
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(eyre!("Habit '{}' does not exist!", self.habit));
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
