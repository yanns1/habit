use crate::db;
use crate::habit::{Day, Habit};
use crate::utils;
use chrono::{DateTime, Datelike, Local, TimeZone, Weekday};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::{Buffer, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Span;
use ratatui::widgets::Widget;
use rusqlite::Connection;
use std::cmp::Ordering;

const N_WEEKS_IN_YEAR: u16 = 53;
const N_DAYS_IN_WEEK: u16 = 7;
const WIDTH_FOR_DAY: u16 = 3;
const HEIGHT_FOR_DAY: u16 = 2;
const WIDTH_FOR_DAY_NAME: u16 = 3;
const HEIGHT_FOR_DAY_NAME: u16 = 2;
const WIDTH_FOR_NAV: u16 = 13;
const HEIGHT_FOR_NAV: u16 = 1;
const WIDTH_FOR_YEAR: u16 = 4;
const HEIGHT_FOR_YEAR: u16 = 1;

#[derive(Debug, Clone, Copy)]
/// The "type" of a day, as we are concerned about when we need to know
/// what to output in each cell of the heatmap.
enum DayType {
    /// A day not in the year considered, either in the previous or the next year.
    NotInYear,
    /// A day to come in the future.
    ToCome,
    /// A day for which the habit needs not be performed/logged.
    ShouldNotHabit,
    /// A day for which the habit needs to be performed/logged.
    /// Contains a boolean indicating whether it was effectively logged or not.
    ShouldHabit(bool),
}

pub struct HeatMap {
    conn: Option<Connection>,

    days_mat: Vec<DayType>,

    today: DateTime<Local>,
    today_year: i32,

    // TODO(yann): We may not want to keep state for the year, instead let the App manage
    // the year, as it could be used for all visualizations.
    // Or maybe we want each visualizer to have its year...
    year: i32,
    start_idx: usize,
    today_idx: usize,
    end_idx: usize,
}

impl HeatMap {
    pub fn new() -> Self {
        // Make a days matrix, a 7 by 53 matrix where each cell corresponds to a day of the year.
        // A cell contains the "type" of the day it corresponds to (see `DayType`).
        let days_mat =
            vec![DayType::NotInYear; (N_WEEKS_IN_YEAR as usize) * (N_DAYS_IN_WEEK as usize)];
        let today = Local::now();
        let today_year = today.year();

        let mut heatmap = HeatMap {
            conn: None,

            days_mat,

            today,
            today_year,

            year: today_year,
            start_idx: 0,
            today_idx: 0,
            end_idx: 0,
        };

        heatmap.update_days_mat_to_year(today_year);

        heatmap
    }

    pub fn update_to_habit_and_year(&mut self, habit: &Habit, year: i32) -> anyhow::Result<()> {
        self.update_days_mat_to_year(year);
        self.update_days_mat_to_habit(habit)
    }

    pub fn update_to_cur_year(&mut self, habit: &Habit) -> anyhow::Result<()> {
        if self.year == self.today_year {
            return Ok(());
        }

        self.update_to_habit_and_year(habit, self.today_year)
    }

    pub fn update_to_next_year(&mut self, habit: &Habit) -> anyhow::Result<()> {
        self.update_to_habit_and_year(habit, self.year + 1)
    }

    pub fn update_to_prev_year(&mut self, habit: &Habit) -> anyhow::Result<()> {
        self.update_to_habit_and_year(habit, self.year - 1)
    }

    pub fn update_to_habit(&mut self, habit: &Habit) -> anyhow::Result<()> {
        self.update_days_mat_to_habit(habit)
    }

    fn update_days_mat_to_year(&mut self, year: i32) {
        let first_day_of_year = Local.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
        let first_weekday_of_year = first_day_of_year.weekday();
        let last_day_of_year = Local.with_ymd_and_hms(year, 12, 31, 0, 0, 0).unwrap();
        let last_weekday_of_year = last_day_of_year.weekday();

        // Set all days in previous year to `DayType::NotInYear`.
        let mut start_idx = 0;
        while Weekday::try_from((start_idx % 7) as u8).unwrap() != first_weekday_of_year {
            self.days_mat[start_idx] = DayType::NotInYear;
            start_idx += 1;
        }

        // Set all days in next year to `DayType::NotInYear`.
        let mut end_idx = self.days_mat.capacity() - 1;
        while Weekday::try_from((end_idx % 7) as u8).unwrap() != last_weekday_of_year {
            self.days_mat[end_idx] = DayType::NotInYear;
            end_idx -= 1;
        }

        // For all days after today, set to `DayType::ToCome`.
        match year.cmp(&self.today_year) {
            Ordering::Less => {}
            Ordering::Equal => {
                let today_idx = start_idx + (utils::nth_day_of_year(&self.today) as usize) - 1;
                for d in self.days_mat[(today_idx + 1)..(end_idx + 1)].iter_mut() {
                    *d = DayType::ToCome;
                }

                self.today_idx = today_idx;
            }
            Ordering::Greater => {
                for d in self.days_mat[(start_idx)..(end_idx + 1)].iter_mut() {
                    *d = DayType::ToCome;
                }
            }
        }

        self.year = year;
        self.start_idx = start_idx;
        self.end_idx = end_idx;
    }

    /// Should be called _after_ `update_days_mat_to_year` has been called, otherwise
    /// fields will not be properly set.
    fn update_days_mat_to_habit(&mut self, habit: &Habit) -> anyhow::Result<()> {
        if self.year > self.today_year {
            return Ok(());
        }

        if self.conn.is_none() {
            self.conn = Some(db::open_db()?);
        }
        let conn = self.conn.as_ref().unwrap();

        let log_datetimes = db::habit_get_logs_for_year(conn, &habit.name, self.year)?;
        let mut log_offsets = log_datetimes
            .iter()
            .map(|datetime| self.start_idx + (utils::nth_day_of_year(datetime) as usize) - 1)
            .collect::<Vec<usize>>();
        log_offsets.sort();

        let mut log_offset_idx: usize = 0;
        let mut weekday = Weekday::try_from((self.start_idx % 7) as u8).unwrap();
        let mut end_idx = self.end_idx;
        if self.year == self.today_year {
            end_idx = self.today_idx;
        }
        for i in self.start_idx..end_idx + 1 {
            self.days_mat[i] = if habit.days.contains(&Day::from(weekday)) {
                if let Some(&offset) = log_offsets.get(log_offset_idx) {
                    if offset < i {
                        log_offset_idx += 1;
                    }

                    if offset == i {
                        DayType::ShouldHabit(true)
                    } else {
                        DayType::ShouldHabit(false)
                    }
                } else {
                    DayType::ShouldHabit(false)
                }
            } else {
                if let Some(&offset) = log_offsets.get(log_offset_idx) {
                    if offset < i {
                        log_offset_idx += 1;
                    }
                }

                DayType::ShouldNotHabit
            };

            weekday = weekday.succ();
        }

        Ok(())
    }
}

impl Default for HeatMap {
    fn default() -> Self {
        HeatMap::new()
    }
}

impl Widget for &mut HeatMap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Layout
        let [_, year_rect, _, days_rect, nav_rect, _] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(HEIGHT_FOR_YEAR),
                Constraint::Length(1),
                Constraint::Length(HEIGHT_FOR_DAY * N_DAYS_IN_WEEK),
                Constraint::Length(HEIGHT_FOR_NAV),
                Constraint::Fill(1),
            ])
            .areas(area);
        let [_, year_rect, _] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(WIDTH_FOR_YEAR),
                Constraint::Fill(1),
            ])
            .areas(year_rect);
        let [_, days_rect, _, days_mat_rect, _] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(WIDTH_FOR_DAY_NAME),
                Constraint::Length(2),
                Constraint::Length(WIDTH_FOR_DAY * N_WEEKS_IN_YEAR),
                Constraint::Fill(1),
            ])
            .areas(days_rect);
        let [_, nav_rect, _] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(WIDTH_FOR_NAV),
                Constraint::Fill(1),
            ])
            .areas(nav_rect);

        // Styles
        // NOTE: Cannot make these constants, because some methods are not const.
        let on_black = Style::new().white().on_black();
        let on_gray = Style::new().black().on_gray();
        let on_dark_gray = Style::new().white().on_dark_gray();
        let on_light_green = Style::new().black().on_light_green();
        let on_light_red = Style::new().white().on_light_red();

        // Render year
        buf.set_span(
            year_rect.x,
            year_rect.y,
            &Span::styled(self.year.to_string(), on_black),
            WIDTH_FOR_YEAR,
        );

        // Render day names
        buf.set_span(
            days_rect.x,
            days_rect.y,
            &Span::styled("Mon", on_black),
            WIDTH_FOR_DAY_NAME,
        );
        buf.set_span(
            days_rect.x,
            days_rect.y + HEIGHT_FOR_DAY_NAME,
            &Span::styled("Tue", on_black),
            WIDTH_FOR_DAY_NAME,
        );
        buf.set_span(
            days_rect.x,
            days_rect.y + 2 * HEIGHT_FOR_DAY_NAME,
            &Span::styled("Wed", on_black),
            WIDTH_FOR_DAY_NAME,
        );
        buf.set_span(
            days_rect.x,
            days_rect.y + 3 * HEIGHT_FOR_DAY_NAME,
            &Span::styled("Thu", on_black),
            WIDTH_FOR_DAY_NAME,
        );
        buf.set_span(
            days_rect.x,
            days_rect.y + 4 * HEIGHT_FOR_DAY_NAME,
            &Span::styled("Fri", on_black),
            WIDTH_FOR_DAY_NAME,
        );
        buf.set_span(
            days_rect.x,
            days_rect.y + 5 * HEIGHT_FOR_DAY_NAME,
            &Span::styled("Sat", on_black),
            WIDTH_FOR_DAY_NAME,
        );
        buf.set_span(
            days_rect.x,
            days_rect.y + 6 * HEIGHT_FOR_DAY_NAME,
            &Span::styled("Sun", on_black),
            WIDTH_FOR_DAY_NAME,
        );

        // Render days matrix
        let mut i = 0;
        let start_x = days_mat_rect.x;
        let end_x = start_x + WIDTH_FOR_DAY * N_WEEKS_IN_YEAR;
        let start_y = days_mat_rect.y;
        let end_y = start_y + HEIGHT_FOR_DAY * N_DAYS_IN_WEEK;
        for x in (start_x..end_x).step_by(WIDTH_FOR_DAY as usize) {
            for y in (start_y..end_y).step_by(HEIGHT_FOR_DAY as usize) {
                let span = match self.days_mat[i] {
                    DayType::NotInYear => None,
                    DayType::ToCome => Some(Span::styled(
                        " ".repeat((WIDTH_FOR_DAY - 1) as usize),
                        on_dark_gray,
                    )),
                    DayType::ShouldNotHabit => Some(Span::styled(
                        if self.year == self.today_year && i == self.today_idx {
                            "ty".to_string()
                        } else {
                            " ".repeat((WIDTH_FOR_DAY - 1) as usize)
                        },
                        if self.year == self.today_year && i == self.today_idx {
                            on_dark_gray
                        } else {
                            on_gray
                        },
                    )),
                    DayType::ShouldHabit(true) => Some(Span::styled(
                        if self.year == self.today_year && i == self.today_idx {
                            "ty".to_string()
                        } else {
                            " ".repeat((WIDTH_FOR_DAY - 1) as usize)
                        },
                        on_light_green,
                    )),
                    DayType::ShouldHabit(false) => Some(Span::styled(
                        if self.year == self.today_year && i == self.today_idx {
                            "ty".to_string()
                        } else {
                            " ".repeat((WIDTH_FOR_DAY - 1) as usize)
                        },
                        on_light_red,
                    )),
                };

                if let Some(span) = span {
                    buf.set_span(x, y, &span, WIDTH_FOR_DAY - 1);
                }

                i += 1;
            }
        }

        // Render nav
        buf.set_span(
            nav_rect.x,
            nav_rect.y,
            &Span::styled("< h | o | l >", on_black),
            WIDTH_FOR_NAV,
        );
    }
}
