use std::str::FromStr;

use crate::engine::Engine;
use crate::habit::{At, Day, Habit, ParseAtError};
use crate::new::cli::NewCli;
use dialoguer::MultiSelect;
use dialoguer::{theme::ColorfulTheme, Input};

pub fn get_engine(_cli: NewCli) -> Box<dyn Engine> {
    Box::new(NewEngine {})
}

struct NewEngine {}

impl Engine for NewEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        let dialoguer_theme = ColorfulTheme::default();

        // ask habit info
        let name = Input::<String>::with_theme(&dialoguer_theme)
            .with_prompt("Name (make it short!)")
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
        println!("{:?}", habit);

        // TODO:
        // add to DB

        Ok(())
    }
}
