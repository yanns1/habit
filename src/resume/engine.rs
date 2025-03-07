use crate::db;
use crate::engine::Engine;
use crate::resume::cli::ResumeCli;
use eyre::eyre;

pub fn get_engine(cli: ResumeCli) -> Box<dyn Engine> {
    Box::new(ResumeEngine { habit: cli.habit })
}

struct ResumeEngine {
    habit: String,
}

impl Engine for ResumeEngine {
    fn run(&mut self) -> eyre::Result<()> {
        let conn = db::get_conn!();

        // Check if habit exists.
        if !db::habit_exists(&conn, &self.habit)? {
            return Err(eyre!("Habit '{}' does not exist!", self.habit));
        }

        // Check if habit is already resumed.
        if !db::habit_is_suspended(&conn, &self.habit)? {
            println!("Habit '{}' is already resumed.", self.habit);
            return Ok(());
        }

        // Resume the habit, i.e. update its suspended value to false in the DB.
        db::habit_update_suspended(&conn, &self.habit, false)?;
        println!("Habit '{}' successfully resumed.", self.habit);

        Ok(())
    }
}
