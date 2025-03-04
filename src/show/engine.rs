use super::heatmap::HeatMap;
use crate::db;
use crate::engine::Engine;
use crate::habit::Habit;
use crate::show::cli::ShowCli;
use crate::tui;
use crate::utils;
use anyhow::anyhow;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind},
    layout::{Direction, Layout, Rect},
    prelude::Constraint,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap,
    },
    Frame,
};
use rusqlite::Connection;
use std::io;

const PRIMARY_COLOR: Color = Color::LightBlue;
const POINTED_LIST_ITEM_STYLE: Style = Style::new().add_modifier(Modifier::BOLD);

pub fn get_engine(cli: ShowCli) -> Box<dyn Engine> {
    Box::new(ShowEngine { habit: cli.habit })
}

struct ShowEngine {
    habit: Option<String>,
}

impl Engine for ShowEngine {
    fn run(&mut self) -> anyhow::Result<()> {
        let conn = db::open_db()?;

        // Check if habit exists in db, if not error.
        if let Some(ref habit_name) = self.habit {
            if !db::habit_exists(&conn, habit_name)? {
                return Err(anyhow!("Habit '{}' does not exist!", habit_name));
            }
        }

        // Prepare init data.
        let habits = db::habit_get_all(&conn)?;
        let init_habit_idx = if let Some(ref habit_name) = self.habit {
            habits
                .iter()
                .position(|habit| habit.name == *habit_name)
                .expect("Initial habit comes from database, so should be within all the habits.")
        } else {
            let habit_name = db::habit_get_name_with_most_recent_log(&conn)?;
            habits
                .iter()
                .position(|habit| habit.name == habit_name)
                .expect("Initial habit comes from database, so should be within all the habits.")
        };

        // Run the TUI.
        let mut terminal = tui::init()?;
        let app_result = App::build(habits, init_habit_idx)?.run(&mut terminal);
        tui::restore(&mut terminal)?;
        app_result?;

        Ok(())
    }
}

struct App {
    conn: Connection,
    key_event: Option<KeyEvent>,
    exit: bool,

    heatmap: HeatMap,

    habits: Vec<Habit>,
    habit_names: Vec<String>,
    habit_list_state: ListState,
    selected_habit_idx: usize,
}

impl App {
    fn build(habits: Vec<Habit>, selected_habit_idx: usize) -> anyhow::Result<Self> {
        debug_assert!((0..habits.len()).contains(&selected_habit_idx));

        let conn = db::open_db()?;

        let habit_names = habits
            .iter()
            .map(|h| h.name.clone())
            .collect::<Vec<String>>();

        let mut habit_list_state = ListState::default();
        habit_list_state.select(Some(selected_habit_idx));

        let mut heatmap = HeatMap::new();
        heatmap.update_to_habit(&habits[selected_habit_idx])?;

        Ok(App {
            conn,
            key_event: None,
            exit: false,

            heatmap,

            habits,
            habit_names,
            habit_list_state,
            selected_habit_idx,
        })
    }

    /// runs the application's main loop until the user quits
    fn run(&mut self, terminal: &mut tui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area())
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // Add a small timeout to the event polling to ensure that the UI
        // remains responsive regardless of whether there are events pending
        // (16ms is ~60fps).
        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                event::Event::Key(key_event) => {
                    self.handle_key_event(key_event);
                }
                _ => {
                    self.key_event = None;
                }
            }
        } else {
            self.key_event = None;
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind == KeyEventKind::Press {
            match key_event.code {
                KeyCode::Char('q') => self.exit(),
                _ => {
                    self.key_event = Some(key_event);
                }
            }
        } else {
            self.key_event = Some(key_event);
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Keyboard input
        // --------------

        if let Some(key_event) = self.key_event {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
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
                        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
                        self.heatmap
                            .update_to_prev_year(&self.habits[self.selected_habit_idx])
                            .unwrap();
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
                        self.heatmap
                            .update_to_next_year(&self.habits[self.selected_habit_idx])
                            .unwrap();
                    }
                    KeyCode::Char('o') => {
                        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
                        self.heatmap
                            .update_to_cur_year(&self.habits[self.selected_habit_idx])
                            .unwrap();
                    }
                    KeyCode::Enter => {
                        self.selected_habit_idx = self
                            .habit_list_state
                            .selected()
                            .expect("There should always be a habit selected.");
                        self.heatmap
                            .update_to_habit(&self.habits[self.selected_habit_idx])
                            .unwrap();
                    }
                    _ => {}
                }
            }
        }

        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
        let selected_habit = &self.habits[self.selected_habit_idx];

        // Layout
        // ------

        let [habit_list_area, habit_details_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(10), Constraint::Fill(1)])
            .areas(area);

        let [habit_details_area, heatmap_area, summary_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Fill(1),
                Constraint::Percentage(10),
            ])
            .areas(habit_details_area);

        // Widgets
        // -------

        // Habit description
        let mut habit_desc = vec![];
        for line in textwrap::wrap(
            &selected_habit.description,
            habit_details_area.width as usize,
        ) {
            habit_desc.push(Line::from(line.to_string()));
        }
        for _ in 0..(habit_details_area.height as usize)
            .saturating_sub(habit_desc.len())
            .saturating_sub(3)
        {
            habit_desc.push(Line::from(""));
        }
        habit_desc.push(Line::from(format!(
            "> Each {} at {}.",
            utils::display_days(&selected_habit.days),
            selected_habit.at
        )));

        let habit_desc_para = Paragraph::new(habit_desc)
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
        let year = self.heatmap.get_year();
        let n_reps_for_year =
            db::habit_get_n_logs_for_year(&self.conn, &selected_habit.name, year).unwrap();
        let n_reps_total = db::habit_get_n_logs(&self.conn, &selected_habit.name).unwrap();
        let summary_lines = vec![
            Line::from(vec![
                Span::from(format!("In {}, you have completed ", year)),
                Span::styled(
                    format!(
                        "{} {}",
                        n_reps_for_year,
                        if n_reps_for_year <= 1 { "rep" } else { "reps" }
                    ),
                    Style::new().bold(),
                ),
                Span::from(format!(
                    " for habit '{}'. {}",
                    selected_habit.name,
                    if n_reps_for_year > 0 {
                        "Congratulations!"
                    } else {
                        ""
                    }
                )),
            ]),
            Line::from(vec![
                Span::from("Since the beginning, you have completed "),
                Span::styled(
                    format!(
                        "{} {}",
                        n_reps_total,
                        if n_reps_total <= 1 { "rep" } else { "reps" }
                    ),
                    Style::new().bold(),
                ),
                Span::from("."),
            ]),
        ];
        let summary_para = Paragraph::new(summary_lines)
            .centered()
            .wrap(Wrap { trim: true });

        // Rendering
        // ---------

        habit_desc_para.render(habit_details_area, buf);
        StatefulWidget::render(habit_list, habit_list_area, buf, &mut self.habit_list_state);
        self.heatmap.render(heatmap_area, buf);
        summary_para.render(summary_area, buf);
    }
}
