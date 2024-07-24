use crate::habit::Habit;
use crate::show::ui::centered_rect;
use ratatui::prelude::{Buffer, Rect};
use ratatui::widgets::{Block, Paragraph, Widget};

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
        let rect = centered_rect(area, 50, 50);
        let para = Paragraph::new(format!("Heatmap for habit {}", self.habit.name))
            .block(Block::bordered());
        para.render(rect, buf);
    }
}
