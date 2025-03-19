use crate::db;
use crate::engine::Engine;
use crate::macros::get_conn;
use crate::suspend::cli::SuspendCli;
use eyre::eyre;

pub fn get_engine(cli: SuspendCli) -> Box<dyn Engine> {
    Box::new(SuspendEngine { habit: cli.habit })
}

struct SuspendEngine {
    habit: String,
}

impl Engine for SuspendEngine {
    fn run(&mut self) -> eyre::Result<()> {
        let conn = get_conn!();

        // Check if habit exists.
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(eyre!("Habit '{}' does not exist!", self.habit));
        }

        // Check if habit is already suspended.
        if db::habit_is_suspended(&conn, &self.habit)? {
            println!("Habit '{}' is already suspended.", self.habit);
            return Ok(());
        }

        // Suspend the habit, i.e. update its suspended value to true in the DB.
        db::habit_update_suspended(&conn, &self.habit, true)?;
        println!("Habit '{}' successfully suspended.", self.habit);

        Ok(())
    }
}
