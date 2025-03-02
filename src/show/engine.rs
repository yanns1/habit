use super::{viz::BowlOfMarbles, viz::HeatMap, viz::Visualizer};
use crate::db;
use crate::engine::Engine;
use crate::habit::Habit;
use crate::show::cli::ShowCli;
use crate::tui;
use anyhow::anyhow;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::prelude::Constraint;
use ratatui::style::Color;
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
    habits: Vec<Habit>,
    habit_names: Vec<String>,
    habit_list_state: ListState,
    selected_habit_idx: usize,

    tabs: [String; 2],
    selected_tab_idx: usize,
    visualizers: [Visualizer; 2],
    heatmap: HeatMap,
    bowl_of_marbles: BowlOfMarbles,

    key_event: Option<KeyEvent>,
    exit: bool,
}

impl App {
    fn build(habits: Vec<Habit>, selected_habit_idx: usize) -> anyhow::Result<Self> {
        debug_assert!((0..habits.len()).contains(&selected_habit_idx));

        let habit_names = habits
            .iter()
            .map(|h| h.name.clone())
            .collect::<Vec<String>>();

        let mut habit_list_state = ListState::default();
        habit_list_state.select(Some(selected_habit_idx));

        let mut heatmap = HeatMap::new();
        heatmap.update_for_habit(&habits[selected_habit_idx])?;

        let bowl_of_marbles = BowlOfMarbles::new();
        // bowl_of_marbles.update_for_habit(&habits[selected_habit_idx]);

        Ok(App {
            habits,
            habit_names,
            habit_list_state,
            selected_habit_idx,

            tabs: ["Heatmap".to_string(), "Bowl of marbles".to_string()],
            selected_tab_idx: 0,
            visualizers: [Visualizer::HeatMap, Visualizer::BowlOfMarbles],
            heatmap,
            bowl_of_marbles,

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

    fn update_selected_vizualizer_for_selected_habit(&mut self) -> anyhow::Result<()> {
        debug_assert!((0..self.visualizers.len()).contains(&self.selected_tab_idx));
        debug_assert!((0..self.habits.len()).contains(&self.selected_habit_idx));
        match self.visualizers[self.selected_tab_idx] {
            Visualizer::HeatMap => {
                self.heatmap
                    .update_for_habit(&self.habits[self.selected_habit_idx])?;
            }
            Visualizer::BowlOfMarbles => {
                // self.bowl_of_marbles.update_for_habit(&self.habits[self.selected_habit_idx])
            }
        };

        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Layout
        // ------

        let [tabs_area, rest] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
            .areas(area);

        let [habit_list_area, viz_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(10), Constraint::Fill(1)])
            .areas(rest);

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
                    KeyCode::Enter => {
                        self.selected_habit_idx = self
                            .habit_list_state
                            .selected()
                            .expect("There should always be a habit selected.");
                        // TODO: Rethink error handling.
                        self.update_selected_vizualizer_for_selected_habit()
                            .unwrap();
                    }
                    KeyCode::Tab => {
                        // Change to next visualizer.
                        self.selected_tab_idx = (self.selected_tab_idx + 1) % self.tabs.len();
                        self.update_selected_vizualizer_for_selected_habit()
                            .unwrap();
                    }
                    KeyCode::BackTab => {
                        // Change to previous visualizer.
                        self.selected_tab_idx =
                            (self.tabs.len() + self.selected_tab_idx - 1) % self.tabs.len();
                        self.update_selected_vizualizer_for_selected_habit()
                            .unwrap();
                    }
                    _ => {}
                }
            }
        }

        // Widgets
        // -------

        // Tabs
        let tabs_block = Block::bordered().title("Visualizations");
        let tabs = Tabs::new(self.tabs.clone())
            .block(tabs_block)
            .style(Style::default().white())
            .highlight_style(PRIMARY_COLOR)
            .select(self.selected_tab_idx);

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

        // Rendering
        // ---------

        tabs.render(tabs_area, buf);
        StatefulWidget::render(habit_list, habit_list_area, buf, &mut self.habit_list_state);

        debug_assert!((0..self.visualizers.len()).contains(&self.selected_tab_idx));
        match self.visualizers[self.selected_tab_idx] {
            Visualizer::HeatMap => self.heatmap.render(viz_area, buf),
            Visualizer::BowlOfMarbles => self.bowl_of_marbles.render(viz_area, buf),
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
