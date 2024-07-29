use crate::habit::Habit;
use crate::utils;
use chrono::{Datelike, Weekday};
use chrono::{TimeZone, Utc};
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

#[derive(Debug, Clone, Copy)]
/// The "type" of a day, as we are concerned about when we need to know
/// what to output in each cell of the heatmap.
enum DayType {
    /// A day not in the year considered, either in the previous or the next year.
    NotInYear,
    /// A day to come in the future.
    ToCome,
    /// A day for which the habit need not be performed/logged.
    ShouldNotHabit,
    /// A day for which the habit need to be performed/logged.
    /// Contains a boolean indicating whether it was effectively logged or not.
    ShouldHabit(bool),
}

/// Add a red background to the span if it corresponds to today,
/// otherwise return as is.
macro_rules! highlight_if_today {
    ($today_idx_opt:expr, $i:expr, $span:expr) => {{
        let mut sp = $span;
        if let Some(today_idx) = $today_idx_opt {
            if $i == today_idx {
                sp = sp.style(Style::new().on_red())
            }
        }
        sp
    }};
}

impl<'a> Widget for HeatMap<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 7 days for 53 weeks, the maximum there can be in a year
        let w = 53;
        let h = 7;

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

        // Make a days matrix, a 7 by 53 matrix where each cell corresponds to a day of the year.
        // A cell contains the "type" of the day it corresponds to (see DayType).
        let mut days_mat: Vec<DayType> = vec![DayType::ShouldNotHabit; (w as usize) * (h as usize)];

        let year = 2024;
        let first_day_of_year = Utc.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
        let first_weekday_of_year = first_day_of_year.weekday();
        let last_day_of_year = Utc.with_ymd_and_hms(year, 12, 31, 0, 0, 0).unwrap();
        let last_weekday_of_year = last_day_of_year.weekday();
        let today = Utc::now();

        // Set all days in previous year to DayType::NotInYear.
        let mut start_idx: usize = 0;
        while Weekday::try_from((start_idx % 7) as u8).unwrap() != first_weekday_of_year {
            days_mat[start_idx] = DayType::NotInYear;
            start_idx += 1;
        }
        // Set all days in next year to DayType::NotInYear.
        let mut end_idx: usize = days_mat.capacity() - 1;
        while Weekday::try_from((end_idx % 7) as u8).unwrap() != last_weekday_of_year {
            days_mat[end_idx] = DayType::NotInYear;
            end_idx -= 1;
        }

        // For all days after today, set to DayType::ToCome.
        let mut today_idx_opt: Option<usize> = None;
        if today.year() == year {
            let today_year_offset = utils::nth_day_of_year(&today);
            today_idx_opt = Some((today_year_offset as usize) - 1);
            for d in days_mat[start_idx + (today_year_offset as usize)..end_idx + 1].iter_mut() {
                *d = DayType::ToCome;
            }
        }

        // TODO: Select logs for habit in year.
        // Use the day number (in year) as an offset into the matrix.
        let done = vec![(1, true), (8, true), (15, false), (22, true)];
        for (i, b) in done {
            days_mat[i] = DayType::ShouldHabit(b);
        }

        let mut i = 0;
        let start_x = rect.x;
        let end_x = start_x + 2 * w; // 2*w because one char for the cell and one space for the gutter
        let start_y = rect.y;
        let end_y = start_y + h;
        for x in (start_x..end_x).step_by(2) {
            for y in start_y..end_y {
                let span = match days_mat[i] {
                    DayType::NotInYear => Span::from("~"),
                    DayType::ToCome => Span::from("?"),
                    DayType::ShouldNotHabit => {
                        highlight_if_today!(today_idx_opt, i, Span::from("_"))
                    }
                    DayType::ShouldHabit(true) => highlight_if_today!(
                        today_idx_opt,
                        i,
                        Span::styled("1", Style::new().green().bold())
                    ),
                    DayType::ShouldHabit(false) => highlight_if_today!(
                        today_idx_opt,
                        i,
                        Span::styled("0", Style::new().red().bold())
                    ),
                };

                buf.set_span(x, y, &span, 1);
                buf.set_span(x + 1, y, &Span::from(" "), 1);
                i += 1;
            }
        }

        // TODO: Make it stateful:
        //  1. Color and show date on hover on cell
        //  2. add events to paginate forward or backward
    }
}
