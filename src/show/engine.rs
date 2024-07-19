use super::{viz::BowlOfMarbles, viz::HeatMap, viz::ProgressVisualizer};
use crate::db;
use crate::engine::Engine;
use crate::habit::Habit;
use crate::show::cli::ShowCli;
use crate::tui;
use anyhow::Context;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::prelude::Constraint;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Tabs;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    style::Stylize,
    widgets::Widget,
    Frame,
};
use std::io;

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
        let habit = match self.habit {
            // if provided, go get data from database to construct a Habit
            Some(ref habit_name) => db::habit_get_by_name(&conn, habit_name)?,
            // if not provided, select the one for which there is the most recent log
            None => {
                let habit_name = conn
                    .query_row(
                        "SELECT habit FROM log ORDER BY created DESC LIMIT 1;
",
                        (),
                        |row| row.get::<usize, String>(0),
                    )
                    .with_context(|| "Failed to select the habit that has the most recent log.")?;

                db::habit_get_by_name(&conn, &habit_name)?
            }
        };

        // Run the TUI
        // -----------
        let mut terminal = tui::init()?;
        let app_result = App::build(habit)?.run(&mut terminal);
        tui::restore(&mut terminal)?;
        app_result?;

        Ok(())
    }
}

#[derive(Debug)]
struct App {
    visualizer: ProgressVisualizer,
    habit: Habit,
    exit: bool,
}

impl App {
    fn build(habit: Habit) -> anyhow::Result<Self> {
        Ok(App {
            visualizer: ProgressVisualizer::HeatMap,
            habit,
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

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size())
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // Add a small timeout to the event polling to ensure that the UI
        // remains responsive regardless of whether there are events pending
        // (16ms is ~60fps).
        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key_evt) = event::read()? {
                if key_evt.kind == KeyEventKind::Press {
                    self.handle_key_event(key_evt);
                }
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_evt: KeyEvent) {
        match key_evt.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Esc => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Layout
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
        let habit_select_area = layout[0];
        let viz_area = layout[1];

        // Widgets
        let tabs = Tabs::new(vec!["Heatmap", "Bowl of marbles"])
            .block(Block::bordered().title("Visualizations"))
            .style(Style::default().white())
            .highlight_style(Style::default().blue())
            .select(0);

        let habit_select = Paragraph::new(self.habit.name.clone()).block(Block::bordered());

        // Rendering
        tabs.render(tabs_area, buf);
        habit_select.render(habit_select_area, buf);
        match self.visualizer {
            ProgressVisualizer::HeatMap => HeatMap::new().render(viz_area, buf),
            ProgressVisualizer::BowlOfMarbles => BowlOfMarbles::new().render(viz_area, buf),
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
