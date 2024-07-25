use super::{viz::BowlOfMarbles, viz::HeatMap, viz::ProgressVisualizer};
use crate::db;
use crate::engine::Engine;
use crate::habit::Habit;
use crate::show::cli::ShowCli;
use crate::tui;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::prelude::Constraint;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::HighlightSpacing;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Tabs;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    style::Stylize,
    widgets::Widget,
    Frame,
};
use std::io;

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

        // Prepare the data
        // ----------------
        let init_habit = match self.habit {
            // if provided, go get data from database to construct a Habit
            Some(ref habit_name) => db::habit_get_by_name(&conn, habit_name)?,
            // if not provided, select the one for which there is the most recent log
            None => db::habit_get_with_most_recent_log(&conn)?,
        };
        let habits = db::habit_get_all(&conn)?;
        let init_habit_idx = habits
            .iter()
            .position(|habit| habit.name == init_habit.name)
            .expect("Initial habit comes from database, so should be within all the habits");

        // Run the TUI
        // -----------
        let mut terminal = tui::init()?;
        let app_result = App::build(habits, init_habit_idx)?.run(&mut terminal);
        tui::restore(&mut terminal)?;
        app_result?;

        Ok(())
    }
}

#[derive(Debug)]
struct App {
    tabs: Vec<String>,
    visualizers: Vec<ProgressVisualizer>,
    selected_tab_idx: usize,
    habits: Vec<Habit>,
    habit_names: Vec<String>,
    selected_habit_idx: usize,
    habit_list_state: ListState,
    key_event: Option<KeyEvent>,
    exit: bool,
}

impl App {
    fn build(habits: Vec<Habit>, selected_habit_idx: usize) -> anyhow::Result<Self> {
        let habit_names = habits
            .iter()
            .map(|h| h.name.clone())
            .collect::<Vec<String>>();

        let mut habit_list_state = ListState::default();
        habit_list_state.select(Some(selected_habit_idx));

        Ok(App {
            tabs: vec!["Heatmap".to_string(), "Bowl of marbles".to_string()],
            visualizers: vec![
                ProgressVisualizer::HeatMap,
                ProgressVisualizer::BowlOfMarbles,
            ],
            selected_tab_idx: 0,
            habits,
            habit_names,
            selected_habit_idx,
            habit_list_state,
            key_event: None,
            exit: false,
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
        frame.render_widget(self, frame.size())
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

    fn next_viz(&mut self) {
        self.selected_tab_idx = (self.selected_tab_idx + 1) % self.tabs.len();
    }

    fn prev_viz(&mut self) {
        self.selected_tab_idx = (self.tabs.len() + self.selected_tab_idx - 1) % self.tabs.len();
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Layout
        // ^^^^^^
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
            .split(area);
        let tabs_area = layout[0];
        let rest = layout[1];

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(10), Constraint::Fill(1)])
            .split(rest);
        let habit_list_area = layout[0];
        let viz_area = layout[1];

        // Change app state depending on received events
        // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        // Update habit list state depending on key event
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
                    KeyCode::Enter => {
                        self.selected_habit_idx = self
                            .habit_list_state
                            .selected()
                            .expect("There should always be a habit selected.");
                    }
                    KeyCode::Tab => self.next_viz(),
                    KeyCode::BackTab => self.prev_viz(),
                    _ => {}
                }
            }
        }

        // Widgets
        // ^^^^^^^
        // Tabs
        let tabs_block = Block::bordered().title("Visualizations");
        let tabs = Tabs::new(self.tabs.clone())
            .block(tabs_block)
            .style(Style::default().white())
            .highlight_style(Style::default().blue())
            .select(self.selected_tab_idx);

        // Habit list
        let habit_list_block = Block::bordered().title("Habits");
        let items: Vec<ListItem> = self
            .habit_names
            .iter()
            .enumerate()
            .map(|(i, habit)| {
                if i == self.selected_habit_idx {
                    ListItem::from(habit.clone()).light_blue()
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

        // Rendering
        // ^^^^^^^^^
        tabs.render(tabs_area, buf);
        StatefulWidget::render(habit_list, habit_list_area, buf, &mut self.habit_list_state);

        // selected_habit_idx should always be within the bounds of habits
        let selected_habit = &self.habits[self.selected_habit_idx];
        // selected_tab_idx should always be within the bounds of visualizers
        match self.visualizers[self.selected_tab_idx] {
            ProgressVisualizer::HeatMap => HeatMap::new(selected_habit).render(viz_area, buf),
            ProgressVisualizer::BowlOfMarbles => {
                BowlOfMarbles::new(selected_habit).render(viz_area, buf)
            }
        }

        // // Show current number of logged reps
        // let n_reps = db::get_n_logs_for_habit(&conn, habit)?;
        // println!(
        //     "You accumulated {} for habit '{}'. {}",
        //     format!("{} {}", n_reps, if n_reps <= 1 { "rep" } else { "reps" }).bold(),
        //     habit,
        //     if n_reps > 0 { "Congratulations!" } else { "" }
        // );
    }
}
