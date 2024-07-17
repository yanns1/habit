use crate::db;
use crate::habit::At;
use crate::habit::Day;
use crate::habit::ParseAtError;
use anyhow::Context;
use dialoguer::Confirm;
use dialoguer::MultiSelect;
use dialoguer::{theme::ColorfulTheme, Input};
use lazy_static::lazy_static;
use std::str::FromStr;

lazy_static! {
    static ref DAYS: [Day; 7] = [
        Day::Monday,
        Day::Tuesday,
        Day::Wednesday,
        Day::Thursday,
        Day::Friday,
        Day::Saturday,
        Day::Sunday,
    ];
}

pub fn prompt_habit_name() -> anyhow::Result<String> {
    let conn = db::open_db()?;
    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();

    let name = Input::<String>::with_theme(&dialoguer_theme)
        .with_prompt("Name (make it short!)")
        .validate_with(|input: &String| -> Result<(), String> {
            // Check that there is no existing habit with the same name
            let input = input.trim();
            match conn.query_row(
                "SELECT name FROM habit WHERE name = ?1",
                rusqlite::params![input],
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

    Ok(name)
}

pub fn prompt_habit_description() -> anyhow::Result<String> {
    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();

    let description = Input::<String>::with_theme(&dialoguer_theme)
        .with_prompt("Description (make it as long as you want)")
        .interact_text()?
        .trim()
        .to_string();

    Ok(description)
}

pub fn prompt_habit_days() -> anyhow::Result<Vec<Day>> {
    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();

    let days = MultiSelect::with_theme(&dialoguer_theme)
        .with_prompt("Days")
        .items(&DAYS[..])
        .interact()?
        .into_iter()
        .map(|i| DAYS[i].clone())
        .collect();

    Ok(days)
}

pub fn prompt_habit_at() -> anyhow::Result<At> {
    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();

    At::from_str(
        Input::<String>::with_theme(&dialoguer_theme)
            .with_prompt("At (hh:mm)")
            .validate_with(|input: &String| -> Result<(), ParseAtError> {
                At::from_str(input).map(|_| ())
            })
            .interact_text()?
            .trim(),
    )
    .with_context(|| "Not possible if validate_with worked correctly.")
}

pub fn ask_for_confirmation(prompt_mess: &str) -> anyhow::Result<bool> {
    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();

    let answer = Confirm::with_theme(&dialoguer_theme)
        .with_prompt(prompt_mess)
        .interact()?;

    Ok(answer)
}
