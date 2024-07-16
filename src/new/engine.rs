use crate::db::DbMapped;
use crate::utils::open_db;
use rusqlite::params;
use rusqlite::Result;
use std::str::FromStr;

use crate::engine::Engine;
use crate::habit::{At, Day, Habit, ParseAtError};
use crate::new::cli::NewCli;
use dialoguer::MultiSelect;
use dialoguer::{theme::ColorfulTheme, Input};

pub fn get_engine(cli: NewCli) -> Box<dyn Engine> {
    let _ = cli;
    Box::new(NewEngine {})
}

struct NewEngine {}

impl Engine for NewEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        let dialoguer_theme = ColorfulTheme::default();
        let conn = open_db()?;

        // ask habit info
        let name = Input::<String>::with_theme(&dialoguer_theme)
            .with_prompt("Name (make it short!)")
            .validate_with(|input: &String| -> Result<(), String> {
                // Check that there is no existing habit with the same name
                let input = input.trim();
                match conn.query_row(
                    "SELECT name FROM habit WHERE name = ?1",
                    params![input],
                    |_| Ok(()),
                ) {
                    Ok(_) => Err(format!("Habit '{}' already exists!", input)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(()),
                    Err(e) => Err(format!(
                        "Query to select habit with name '{}' failed.\n{}",
                        input, e
                    )),
                }
            })
            .interact_text()?
            .trim()
            .to_string();

        let description = Input::<String>::with_theme(&dialoguer_theme)
            .with_prompt("Description (make it as long as you want)")
            .interact_text()?
            .trim()
            .to_string();

        let days = [
            Day::Monday,
            Day::Tuesday,
            Day::Wednesday,
            Day::Thursday,
            Day::Friday,
            Day::Saturday,
            Day::Sunday,
        ];
        let selected_days = MultiSelect::with_theme(&dialoguer_theme)
            .with_prompt("Days")
            .items(&days[..])
            .defaults(&[false, false, false, false, false, false, false][..])
            .interact()?
            .into_iter()
            .map(|i| days[i].clone())
            .collect();

        let at = Input::<String>::with_theme(&dialoguer_theme)
            .with_prompt("At (hh:mm)")
            .validate_with(|input: &String| -> Result<(), ParseAtError> {
                At::from_str(input).map(|_| ())
            })
            .interact_text()?
            .trim()
            .to_string();

        let habit = Habit::build(name, description, selected_days, &at).unwrap();

        // add to DB
        habit.insert(&conn)?;

        println!("Habit '{}' successfully created!", habit.name);
        println!("Run 'habit log {}' to log progress.", habit.name);
        println!("Run 'habit show {}' to show progress.", habit.name);

        Ok(())
    }
}
