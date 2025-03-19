use crate::db;
use crate::engine::Engine;
use crate::habit::Habit;
use crate::macros::get_conn;
use crate::show::calendar::Calendar;
use crate::show::cli::ShowCli;
use crate::tui;
use crate::utils;
use crate::TODAY;
use chrono::Datelike;
use eyre::eyre;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::prelude::Constraint;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::HighlightSpacing;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use std::collections::hash_map;
use std::collections::HashMap;

const PRIMARY_COLOR: Color = Color::LightBlue;
const POINTED_LIST_ITEM_STYLE: Style = Style::new().add_modifier(Modifier::BOLD);

pub fn get_engine(cli: ShowCli) -> Box<dyn Engine> {
    Box::new(ShowEngine { habit: cli.habit })
}

struct ShowEngine {
    habit: Option<String>,
}

impl Engine for ShowEngine {
    fn run(&mut self) -> eyre::Result<()> {
        let conn = get_conn!();

        // Check if habit exists in db, if not error.
        if let Some(ref habit_name) = self.habit {
            if !db::habit_exists(&conn, habit_name)? {
                return Err(eyre!("Habit '{}' does not exist!", habit_name));
            }
        }

        let habits = db::habit_get_all(&conn)?;

        // If no habits, no reason to open the TUI.
        if habits.is_empty() {
            println!("You have no habits yet! Create one using `habit new`.");
            return Ok(());
        }

        // Prepare init data.
        let init_habit_idx = if let Some(ref habit_name) = self.habit {
            habits
                .iter()
                .position(|habit| habit.name == *habit_name)
                .expect("Initial habit comes from database, so should be within all the habits.")
        } else {
            let habit_name = if !db::log_table_is_empty(&conn)? {
                &db::habit_get_name_with_most_recent_log(&conn)?
            } else {
                &habits[0].name
            };

            habits
                .iter()
                .position(|habit| habit.name == *habit_name)
                .expect("Initial habit comes from database, so should be within all the habits.")
        };

        // Run the TUI.
        let mut app = App::build(conn, habits, init_habit_idx)?;
        let mut terminal = tui::init()?;
        let result = app.run(&mut terminal);
        tui::restore(&mut terminal)?;
        result?;

        Ok(())
    }
}

struct App {
    conn: PooledConnection<SqliteConnectionManager>,

    exit: bool,
    show_help_dialog: bool,

    calendar: Calendar,

    habits: Vec<Habit>,
    habit_names: Vec<String>,
    habit_list_state: ListState,
    selected_habit_idx: usize,

    year: i32,
    cur_year: i32,

    // Key is (<selected_habit_idx>, <year>).
    n_reps_for_year: HashMap<(usize, i32), usize>,
    n_habit_days_for_year: HashMap<(usize, i32), usize>,
    // Key is <selected_habit_idx>.
    n_reps_total: HashMap<usize, usize>,
    n_habit_days_total: HashMap<usize, usize>,
    current_streak: HashMap<usize, u32>,
    longest_streak: HashMap<usize, u32>,
}

impl App {
    fn build(
        conn: PooledConnection<SqliteConnectionManager>,
        habits: Vec<Habit>,
        selected_habit_idx: usize,
    ) -> eyre::Result<Self> {
        debug_assert!((0..habits.len()).contains(&selected_habit_idx));

        let habit_names = habits
            .iter()
            .map(|h| h.name.clone())
            .collect::<Vec<String>>();

        let mut habit_list_state = ListState::default();
        habit_list_state.select(Some(selected_habit_idx));

        let cur_year = TODAY.year();

        let mut n_reps_for_year = HashMap::new();
        n_reps_for_year.insert(
            (selected_habit_idx, cur_year),
            db::habit_get_n_logs_for_year(&conn, &habits[selected_habit_idx].name, cur_year)?,
        );

        let mut n_habit_days_for_year = HashMap::new();
        n_habit_days_for_year.insert(
            (selected_habit_idx, cur_year),
            habits[selected_habit_idx].get_n_habit_days_within_year(cur_year)?,
        );

        let mut n_reps_total = HashMap::new();
        n_reps_total.insert(
            selected_habit_idx,
            db::habit_get_n_logs(&conn, &habits[selected_habit_idx].name)?,
        );

        let mut n_habit_days_total = HashMap::new();
        n_habit_days_total.insert(
            selected_habit_idx,
            habits[selected_habit_idx].get_n_habit_days_since_creation()?,
        );

        let mut current_streak = HashMap::new();
        current_streak.insert(
            selected_habit_idx,
            habits[selected_habit_idx].get_current_streak()?,
        );

        let mut longest_streak = HashMap::new();
        longest_streak.insert(
            selected_habit_idx,
            habits[selected_habit_idx].get_longest_streak()?,
        );

        let mut calendar = Calendar::new();
        calendar.update_to_habit_and_year(&habits[selected_habit_idx], cur_year)?;

        Ok(App {
            conn,

            exit: false,
            show_help_dialog: false,

            calendar,

            habits,
            habit_names,
            habit_list_state,
            selected_habit_idx,

            year: cur_year,
            cur_year,

            n_reps_for_year,
            n_habit_days_for_year,
            n_reps_total,
            n_habit_days_total,
            current_streak,
            longest_streak,
        })
    }

    /// runs the application's main loop until the user quits
    fn run(&mut self, terminal: &mut tui::Tui) -> eyre::Result<()> {
        while !self.exit {
            terminal.draw(|frame| frame.render_widget(&mut *self, frame.area()))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> eyre::Result<()> {
        // Add a small timeout to the event polling to ensure that the UI
        // remains responsive regardless of whether there are events pending
        // (16ms is ~60fps).
        if !event::poll(std::time::Duration::from_millis(16))? {
            return Ok(());
        }

        if let event::Event::Key(key_event) = event::read()? {
            if key_event.kind != KeyEventKind::Press {
                return Ok(());
            }

            match key_event.code {
                KeyCode::Char('q') => {
                    self.exit = true;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.habit_list_state.select_next();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.habit_list_state.select_previous();
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    self.habit_list_state.select_first();
                }
                KeyCode::Char('G') | KeyCode::End => {
                    self.habit_list_state.select_last();
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    if self.year > 1678 {
                        self.year -= 1;

                        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
                        self.calendar.update_to_habit_and_year(
                            &self.habits[self.selected_habit_idx],
                            self.year,
                        )?;

                        let k = (self.selected_habit_idx, self.year);
                        if let hash_map::Entry::Vacant(e) = self.n_reps_for_year.entry(k) {
                            e.insert(db::habit_get_n_logs_for_year(
                                &self.conn,
                                &self.habits[self.selected_habit_idx].name,
                                self.year,
                            )?);
                        }
                        if let hash_map::Entry::Vacant(e) = self.n_habit_days_for_year.entry(k) {
                            e.insert(
                                self.habits[self.selected_habit_idx]
                                    .get_n_habit_days_within_year(self.year)?,
                            );
                        }
                    }
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    if self.year < 2261 {
                        self.year += 1;

                        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
                        self.calendar.update_to_habit_and_year(
                            &self.habits[self.selected_habit_idx],
                            self.year,
                        )?;

                        let k = (self.selected_habit_idx, self.year);
                        if let hash_map::Entry::Vacant(e) = self.n_reps_for_year.entry(k) {
                            e.insert(db::habit_get_n_logs_for_year(
                                &self.conn,
                                &self.habits[self.selected_habit_idx].name,
                                self.year,
                            )?);
                        }
                        if let hash_map::Entry::Vacant(e) = self.n_habit_days_for_year.entry(k) {
                            e.insert(
                                self.habits[self.selected_habit_idx]
                                    .get_n_habit_days_within_year(self.year)?,
                            );
                        }
                    }
                }
                KeyCode::Char('o') => {
                    if self.year != self.cur_year {
                        self.year = self.cur_year;

                        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
                        self.calendar.update_to_habit_and_year(
                            &self.habits[self.selected_habit_idx],
                            self.year,
                        )?;

                        let k = (self.selected_habit_idx, self.year);
                        if let hash_map::Entry::Vacant(e) = self.n_reps_for_year.entry(k) {
                            e.insert(db::habit_get_n_logs_for_year(
                                &self.conn,
                                &self.habits[self.selected_habit_idx].name,
                                self.year,
                            )?);
                        }
                        if let hash_map::Entry::Vacant(e) = self.n_habit_days_for_year.entry(k) {
                            e.insert(
                                self.habits[self.selected_habit_idx]
                                    .get_n_habit_days_within_year(self.year)?,
                            );
                        }
                    }
                }
                KeyCode::Enter => {
                    let prev_selected_habit_idx = self.selected_habit_idx;
                    let new_selected_habit_idx = self
                        .habit_list_state
                        .selected()
                        .expect("There should always be a habit selected.");

                    if new_selected_habit_idx != prev_selected_habit_idx {
                        self.selected_habit_idx = new_selected_habit_idx;

                        self.calendar
                            .update_to_habit(&self.habits[self.selected_habit_idx])?;

                        let k = (self.selected_habit_idx, self.year);
                        if let hash_map::Entry::Vacant(e) = self.n_reps_for_year.entry(k) {
                            e.insert(db::habit_get_n_logs_for_year(
                                &self.conn,
                                &self.habits[self.selected_habit_idx].name,
                                self.year,
                            )?);
                        }
                        if let hash_map::Entry::Vacant(e) = self.n_habit_days_for_year.entry(k) {
                            e.insert(
                                self.habits[self.selected_habit_idx]
                                    .get_n_habit_days_within_year(self.year)?,
                            );
                        }

                        if let hash_map::Entry::Vacant(e) =
                            self.n_reps_total.entry(self.selected_habit_idx)
                        {
                            e.insert(db::habit_get_n_logs(
                                &self.conn,
                                &self.habits[self.selected_habit_idx].name,
                            )?);
                        }
                        if let hash_map::Entry::Vacant(e) =
                            self.n_habit_days_total.entry(self.selected_habit_idx)
                        {
                            e.insert(
                                self.habits[self.selected_habit_idx]
                                    .get_n_habit_days_since_creation()?,
                            );
                        }

                        if let hash_map::Entry::Vacant(e) =
                            self.current_streak.entry(self.selected_habit_idx)
                        {
                            e.insert(self.habits[self.selected_habit_idx].get_current_streak()?);
                        }
                        if let hash_map::Entry::Vacant(e) =
                            self.longest_streak.entry(self.selected_habit_idx)
                        {
                            e.insert(self.habits[self.selected_habit_idx].get_longest_streak()?);
                        }
                    }
                }
                KeyCode::Char('?') => {
                    self.show_help_dialog = !self.show_help_dialog;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
        let selected_habit = &self.habits[self.selected_habit_idx];

        // Layout
        // ------

        let [habit_list_area, habit_details_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(10), Constraint::Fill(1)])
            .areas(area);

        let [habit_details_area, calendar_area, summary_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Fill(1),
                Constraint::Percentage(20),
            ])
            .areas(habit_details_area);

        let [_, summary_area, _] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Percentage(50),
                Constraint::Fill(1),
            ])
            .areas(summary_area);

        // Widgets
        // -------

        // Habit details
        let mut habit_details = vec![];
        for line in textwrap::wrap(
            &selected_habit.description,
            habit_details_area.width as usize,
        ) {
            habit_details.push(Line::from(line.to_string()));
        }
        for _ in 0..(habit_details_area.height as usize)
            .saturating_sub(habit_details.len())
            .saturating_sub(if selected_habit.suspended { 5 } else { 4 })
        {
            habit_details.push(Line::from(""));
        }
        if selected_habit.suspended {
            habit_details.push(Line::from("(suspended)").style(Style::new().italic()))
        }
        habit_details.push(Line::from(vec![
            Span::from(">").style(Style::new().dark_gray()),
            Span::from(format!(
                " Each {} at {}.",
                utils::display_days(&selected_habit.days),
                selected_habit.at
            )),
        ]));
        habit_details
            .push(Line::from(format!("Created at {}.", selected_habit.created_at)).italic());

        let habit_details_para = Paragraph::new(habit_details)
            .block(Block::bordered().title("Habit details"))
            .style(Style::new().white().on_black())
            .wrap(Wrap { trim: true });

        // Habit list
        let habit_list_block = Block::bordered().title("Habits");
        let items: Vec<ListItem> = self
            .habit_names
            .iter()
            .enumerate()
            .map(|(i, habit)| {
                if i == self.selected_habit_idx {
                    ListItem::from(habit.clone()).style(PRIMARY_COLOR)
                } else {
                    ListItem::from(habit.clone())
                }
            })
            .collect();
        let habit_list = List::new(items)
            .block(habit_list_block)
            .highlight_style(POINTED_LIST_ITEM_STYLE)
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        // Summary paragraph
        let n_reps_total = self.n_reps_total[&self.selected_habit_idx];
        let n_habit_days_total = self.n_habit_days_total[&self.selected_habit_idx];
        let percentage_total = if n_habit_days_total != 0 {
            ((n_reps_total as f32) / (n_habit_days_total as f32)) * 100.0
        } else {
            // NOTE: Default to 100% if no habit days.
            // Perhaps a confusing output.
            100.0
        };
        let n_reps_for_year = self.n_reps_for_year[&(self.selected_habit_idx, self.year)];
        let n_habit_days_for_year =
            self.n_habit_days_for_year[&(self.selected_habit_idx, self.year)];
        let percentage_for_year = if n_habit_days_for_year != 0 {
            ((n_reps_for_year as f32) / (n_habit_days_for_year as f32)) * 100.0
        } else {
            // NOTE: Default to 100% if no habit days.
            // Perhaps a confusing output.
            100.0
        };
        let summary_para = Paragraph::new(vec![
            Line::from(format!("Total reps: {}", n_reps_total,)),
            Line::from(format!("Total percentage: {:.1}%", percentage_total)),
            Line::from(format!("Total reps for year: {}", n_reps_for_year,)),
            Line::from(format!(
                "Total percentage for year: {:.1}%",
                percentage_for_year
            )),
            Line::from(format!(
                "Current streak: {}",
                self.current_streak[&self.selected_habit_idx]
            )),
            Line::from(format!(
                "Longest streak: {}",
                self.longest_streak[&self.selected_habit_idx]
            )),
        ])
        .block(Block::bordered().title("Summary"))
        .wrap(Wrap { trim: true });

        // Rendering
        // ---------

        habit_details_para.render(habit_details_area, buf);
        StatefulWidget::render(habit_list, habit_list_area, buf, &mut self.habit_list_state);
        self.calendar.render(calendar_area, buf);
        summary_para.render(summary_area, buf);

        if self.show_help_dialog {
            let [_, help_area, _] = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Percentage(40),
                    Constraint::Fill(1),
                ])
                .areas(area);
            let [_, help_area, _] = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Percentage(50),
                    Constraint::Fill(1),
                ])
                .areas(help_area);

            let help_lines = vec![
                Line::from(
                    "Press: 'q'     to quit                       '?'   to toggle this help dialog",
                ),
                Line::from(
                    "       'j'     to select next habit          'k'   to select previous habit",
                ),
                Line::from(
                    "       'g'     to select first habit         'G'   to select last habit",
                ),
                Line::from("       'Enter' to confirm habit selection"),
                Line::from("       'h'     to show previous year         'l'   to show next year"),
                Line::from("       'o'     to go back to current year."),
            ];
            let help_para = Paragraph::new(help_lines)
                .block(Block::bordered().title("Help"))
                .wrap(Wrap { trim: false });

            // Clear the help area first.
            let clear = Clear;
            clear.render(help_area, buf);
            help_para.render(help_area, buf);
        }
    }
}
