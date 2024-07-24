use crate::habit::Habit;
use crate::show::ui::centered_rect;
use ratatui::prelude::{Buffer, Rect};
use ratatui::widgets::{Block, Paragraph, Widget};

pub struct BowlOfMarbles<'a> {
    habit: &'a Habit,
}

impl<'a> BowlOfMarbles<'a> {
    pub fn new(habit: &'a Habit) -> Self {
        BowlOfMarbles { habit }
    }
}

impl<'a> Widget for BowlOfMarbles<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rect = centered_rect(area, 50, 50);
        let para = Paragraph::new(format!("Bowl of marbles for habit {}", self.habit.name))
            .block(Block::bordered());
        para.render(rect, buf);
    }
}
