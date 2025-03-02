use crate::show::ui::centered_rect;
use ratatui::prelude::{Buffer, Rect};
use ratatui::widgets::{Block, Paragraph, Widget};

pub struct BowlOfMarbles {}

impl BowlOfMarbles {
    pub fn new() -> Self {
        BowlOfMarbles {}
    }
}

impl Default for BowlOfMarbles {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &mut BowlOfMarbles {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rect = centered_rect(area, 50, 50);
        let para =
            Paragraph::new(format!("Bowl of marbles for habit {}", "?")).block(Block::bordered());
        para.render(rect, buf);
    }
}
