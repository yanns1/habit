use crate::habit::Habit;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::{Buffer, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Span;
use ratatui::widgets::Widget;

pub struct HeatMap<'a> {
    habit: &'a Habit,
}

impl<'a> HeatMap<'a> {
    pub fn new(habit: &'a Habit) -> Self {
        HeatMap { habit }
    }
}

impl<'a> Widget for HeatMap<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let w = 30;
        let h = 12;

        // Make a centered rect for the heatmap,
        // leveraging our knowledge of the exact number
        // of rows and columns it will have.
        let [_, rect, _] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(h),
                Constraint::Fill(1),
            ])
            .areas(area);
        let [_, rect, _] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(2 * w),
                Constraint::Fill(1),
            ])
            .areas(rect);

        // Split our centered rect into rows.
        let mut vertical_constraints = Vec::with_capacity((h as usize) + 2);
        vertical_constraints.push(Constraint::Fill(1));
        for _ in 0..h {
            vertical_constraints.push(Constraint::Length(1));
        }
        vertical_constraints.push(Constraint::Fill(1));
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vertical_constraints)
            .split(rect);

        // Matrix filled with booleans representing whether the habit
        // was logged on a given day.
        // TODO: Replace with real data!
        let mut done = Vec::with_capacity((w as usize) * (h as usize));
        for i in 0..w * h {
            if i % 2 == 0 {
                done.push(true);
            } else {
                done.push(false);
            }
        }

        let mut i = 0;
        for row in &rows[1..rows.len() - 1] {
            // Prepare a line as a sequence of spans.
            let mut spans = Vec::with_capacity(w as usize);
            for _ in 0..w - 1 {
                spans.push(if done[i] {
                    Span::styled("✓ ", Style::new().green().bold())
                } else {
                    Span::styled("❌", Style::new().red().bold())
                });
                i += 1;
            }
            spans.push(if done[i] {
                Span::styled("✓ ", Style::new().green().bold())
            } else {
                Span::styled("❌", Style::new().red().bold())
            });
            i += 1;

            // Render the line/row.
            buf.set_line(row.x, row.y, &spans.into(), row.width);
        }

        // TODO: Make it stateful:
        //  1. Color and show date on hover on cell
        //  2. add events to paginate forward or backward
    }
}
