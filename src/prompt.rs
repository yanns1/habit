use crate::habit::At;
use crate::habit::Day;
use crate::habit::ParseAtError;
use crate::macros::get_conn;
use crossterm::cursor::MoveToPreviousLine;
use crossterm::execute;
use crossterm::style::Stylize;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use dialoguer::Input;
use dialoguer::MultiSelect;
use eyre::WrapErr;
use std::io;
use std::str::FromStr;

const DAYS: [Day; 7] = [
    Day::Monday,
    Day::Tuesday,
    Day::Wednesday,
    Day::Thursday,
    Day::Friday,
    Day::Saturday,
    Day::Sunday,
];

pub fn prompt_habit_name() -> eyre::Result<String> {
    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();

    let name = Input::<String>::with_theme(&dialoguer_theme)
        .with_prompt("Name (make it short!)")
        .validate_with(|input: &String| -> Result<(), String> {
            // Check that there is no existing habit with the same name
            let conn = get_conn!();
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

pub fn prompt_habit_description() -> eyre::Result<String> {
    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();

    let description = Input::<String>::with_theme(&dialoguer_theme)
        .with_prompt("Description (make it as long as you want)")
        .interact_text()?
        .trim()
        .to_string();

    Ok(description)
}

pub fn prompt_habit_days() -> eyre::Result<Vec<Day>> {
    // NOTE: I need to block the user from selecting no days. However, dialoguer's `MultiSelect`
    // allows for no selection, and there is no way to register a "validate" callback like other
    // components allow.
    //
    // It likely is rather easy to add that feature to dialoguer, but I decided to not bother
    // making a pull request, as there is no activity on the Github repo since 10 months (despite
    // many pending issus and pull requests).
    //
    // So I hacked my way around the `MultiSelect`. First, I disabled the clearing and reporting
    // behavior of `MultiSelect`, to control how and when to clear. Then I intersect my validation
    // logic, and print an error in case nothing is selected, following the style of the theme I
    // use: `ColorfulTheme`. I found inspiration in dialoguer's source code.
    //
    // This code obviously is brittle. as changes to `MultiSelect` output or `ColorfulTheme` will
    // quickly have a desastrous effect.

    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();
    let error_prefix = "✘".to_string().red();
    let success_prefix = "✔".to_string().green();
    let success_suffix = "·".to_string().dark_grey();
    let prompt = "Days";
    let multiselect = MultiSelect::with_theme(&dialoguer_theme)
        .clear(false)
        .report(false)
        .with_prompt(prompt)
        .items(&DAYS[..]);

    let mut stdout = io::stdout();
    let mut first_it = true;
    let mut day_idxs = multiselect.clone().interact()?;
    while day_idxs.is_empty() {
        // Clear the multiselect menu, which has
        // 1 line for "? Day >" and 1 line per day.
        let n_lines_to_clear = 8 + if first_it { 0 } else { 1 };
        execute!(
            stdout,
            MoveToPreviousLine(n_lines_to_clear),
            Clear(ClearType::FromCursorDown)
        )
        .wrap_err(format!(
            "Failed to clear the last {} lines.",
            n_lines_to_clear
        ))?;

        eprintln!(
            "{} {}",
            error_prefix,
            "Please select at least one day.".red()
        );
        day_idxs = multiselect.clone().interact()?;

        first_it = false;
    }

    // Clear the multiselect menu one last time.
    let n_lines_to_clear = 8 + if first_it { 0 } else { 1 };
    execute!(
        stdout,
        MoveToPreviousLine(n_lines_to_clear),
        Clear(ClearType::FromCursorDown)
    )
    .wrap_err(format!(
        "Failed to clear the last {} lines.",
        n_lines_to_clear
    ))?;

    // Report as `MultiSelect::interact` would.
    eprint!("{} {} {} ", success_prefix, prompt.bold(), success_suffix);
    let days: Vec<Day> = day_idxs.into_iter().map(|i| DAYS[i].clone()).collect();
    let day_strings: Vec<String> = days.iter().map(|d| d.to_string()).collect();
    for (idx, day_string) in day_strings.into_iter().enumerate() {
        eprint!("{}{}", if idx == 0 { "" } else { ", " }, day_string.green());
    }
    eprint!("\n");

    Ok(days)
}

pub fn prompt_habit_at() -> eyre::Result<At> {
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
    .wrap_err("Not possible if validate_with worked correctly.")
}

pub fn ask_for_confirmation(prompt_mess: &str) -> eyre::Result<bool> {
    let dialoguer_theme: ColorfulTheme = ColorfulTheme::default();

    let answer = Confirm::with_theme(&dialoguer_theme)
        .with_prompt(prompt_mess)
        .interact()?;

    Ok(answer)
}
