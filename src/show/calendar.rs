use crate::db;
use crate::habit::Habit;
use crate::utils;
use crate::TODAY;
use chrono::Datelike;
use chrono::Days;
use chrono::Local;
use chrono::TimeZone;
use chrono::Weekday;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Span;
use ratatui::widgets::Widget;
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
const HEIGHT_FOR_MONTH: u16 = 1;
const MONTHS: [&'static str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

#[derive(Debug, Clone, Copy)]
enum DayKind {
    /// A day to come in the future.
    ToCome,
    /// A day for which the habit needs not be performed/logged.
    ShouldNotHabit,
    /// A day for which the habit needs to be performed/logged.
    /// Contains a boolean indicating whether it was effectively logged or not.
    ShouldHabit(bool),
}

#[derive(Debug, Clone, Copy)]
/// A cell of the days matrix.
enum Cell {
    /// A day not in the year considered, either in the previous or the next year.
    NotInYear,
    /// A day in the year.
    InYear { num: u32, kind: DayKind },
}

pub struct Calendar {
    conn: Option<PooledConnection<SqliteConnectionManager>>,

    days_mat: Vec<Cell>,

    year: i32,
    cur_year: i32,

    start_idx: usize,
    today_idx: usize,
    end_idx: usize,
}

impl Calendar {
    pub fn new() -> Self {
        // Make a days matrix, a 7 by 53 matrix where each cell corresponds to a day of the year.
        let days_mat =
            vec![Cell::NotInYear; (N_WEEKS_IN_YEAR as usize) * (N_DAYS_IN_WEEK as usize)];

        let cur_year = TODAY.year();

        let mut calendar = Calendar {
            conn: None,

            days_mat,

            year: 0,
            cur_year,

            start_idx: 0,
            today_idx: 0,
            end_idx: 0,
        };

        // TDOO: Change that unwrap.
        calendar.update_days_mat_to_year(cur_year).unwrap();

        calendar
    }

    pub fn update_to_habit_and_year(&mut self, habit: &Habit, year: i32) -> eyre::Result<()> {
        self.update_days_mat_to_year(year)?;
        self.update_days_mat_to_habit(habit)
    }

    pub fn update_to_habit(&mut self, habit: &Habit) -> eyre::Result<()> {
        self.update_days_mat_to_habit(habit)
    }

    fn update_days_mat_to_year(&mut self, year: i32) -> eyre::Result<()> {
        // NOTE: Use 12 hours in order to avoid having `checked_add_days` fail, because
        // of daylight saving time transition. The assumption is that such transitions
        // typically are around midnight, or early in the morning.
        let first_day_of_year = Local.with_ymd_and_hms(year, 1, 1, 12, 0, 0).unwrap();
        let first_weekday_of_year = first_day_of_year.weekday();
        let last_day_of_year = Local.with_ymd_and_hms(year, 12, 31, 12, 0, 0).unwrap();
        let last_weekday_of_year = last_day_of_year.weekday();

        // Set all days in previous year to `Cell::NotInYear`.
        let mut start_idx = 0;
        while Weekday::try_from((start_idx % 7) as u8).unwrap() != first_weekday_of_year {
            self.days_mat[start_idx] = Cell::NotInYear;
            start_idx += 1;
        }

        // Set all days in next year to `Cell::NotInYear`.
        let mut end_idx = self.days_mat.capacity() - 1;
        while Weekday::try_from((end_idx % 7) as u8).unwrap() != last_weekday_of_year {
            self.days_mat[end_idx] = Cell::NotInYear;
            end_idx -= 1;
        }

        let one_day = Days::new(1);
        match year.cmp(&self.cur_year) {
            Ordering::Less => {
                let mut dt = first_day_of_year;
                for d in self.days_mat[(start_idx)..(end_idx + 1)].iter_mut() {
                    *d = Cell::InYear {
                        num: dt.day(),
                        // Can be whatever value, because it will/should be overwritten by
                        // `update_days_mat_for_habit`.
                        kind: DayKind::ShouldNotHabit,
                    };

                    dt = dt
                        .checked_add_days(one_day)
                        .ok_or(eyre::eyre!("Failed to add one day to '{}'. Probably a daylight saving time transition.", dt))?;
                }
            }
            Ordering::Equal => {
                let today_idx = start_idx + (utils::nth_day_of_year(&TODAY) as usize) - 1;
                let mut dt = first_day_of_year;

                // Set all days num for days between `start_idx` and `today_idx`.
                for d in self.days_mat[(start_idx)..(today_idx + 1)].iter_mut() {
                    *d = Cell::InYear {
                        num: dt.day(),
                        // Can be whatever value, because it will/should be overwritten by
                        // `update_days_mat_for_habit`.
                        kind: DayKind::ShouldNotHabit,
                    };

                    dt = dt
                        .checked_add_days(one_day)
                        .ok_or(eyre::eyre!("Failed to add one day to '{}'. Probably a daylight saving time transition.", dt))?;
                }

                // For all days after today, set to `Cell::ToCome`.
                for d in self.days_mat[(today_idx + 1)..(end_idx + 1)].iter_mut() {
                    *d = Cell::InYear {
                        num: dt.day(),
                        kind: DayKind::ToCome,
                    };

                    dt = dt
                        .checked_add_days(one_day)
                        .ok_or(eyre::eyre!("Failed to add one day to '{}'. Probably a daylight saving time transition.", dt))?;
                }

                self.today_idx = today_idx;
            }
            Ordering::Greater => {
                let mut dt = first_day_of_year;
                for d in self.days_mat[(start_idx)..(end_idx + 1)].iter_mut() {
                    *d = Cell::InYear {
                        num: dt.day(),
                        kind: DayKind::ToCome,
                    };

                    dt = dt
                        .checked_add_days(one_day)
                        .ok_or(eyre::eyre!("Failed to add one day to '{}'. Probably a daylight saving time transition.", dt))?;
                }
            }
        }

        self.year = year;
        self.start_idx = start_idx;
        self.end_idx = end_idx;

        Ok(())
    }

    /// Should be called _after_ `update_days_mat_to_year` has been called, otherwise
    /// fields will not be properly set.
    fn update_days_mat_to_habit(&mut self, habit: &Habit) -> eyre::Result<()> {
        if self.year > self.cur_year {
            return Ok(());
        }

        if self.conn.is_none() {
            self.conn = Some(db::get_conn!());
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
        if self.year == self.cur_year {
            end_idx = self.today_idx;
        }
        for i in self.start_idx..end_idx + 1 {
            match self.days_mat[i] {
                Cell::NotInYear => {
                    return Err(eyre::eyre!(
                        "Cell should be `InYear`, if `update_days_mat_for_year` was called before."
                    ))
                }
                Cell::InYear {
                    num: _,
                    ref mut kind,
                } => {
                    if habit.days.contains(&weekday.into()) {
                        if let Some(&offset) = log_offsets.get(log_offset_idx) {
                            if offset < i {
                                log_offset_idx += 1;
                            }

                            if offset == i {
                                *kind = DayKind::ShouldHabit(true);
                            } else {
                                *kind = DayKind::ShouldHabit(false);
                            }
                        } else {
                            *kind = DayKind::ShouldHabit(false);
                        }
                    } else {
                        if let Some(&offset) = log_offsets.get(log_offset_idx) {
                            if offset < i {
                                log_offset_idx += 1;
                            }
                        }

                        *kind = DayKind::ShouldNotHabit;
                    }
                }
            };

            weekday = weekday.succ();
        }

        Ok(())
    }
}

impl Default for Calendar {
    fn default() -> Self {
        Calendar::new()
    }
}

impl Widget for &mut Calendar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Layout
        // ------

        let [_, year_rect, _, months_rect, _, days_rect, nav_rect, _] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(HEIGHT_FOR_YEAR),
                Constraint::Length(1),
                Constraint::Length(HEIGHT_FOR_MONTH),
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

        let [_, _, _, months_rect, _] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(WIDTH_FOR_DAY_NAME),
                Constraint::Length(2),
                Constraint::Length(WIDTH_FOR_DAY * N_WEEKS_IN_YEAR),
                Constraint::Fill(1),
            ])
            .areas(months_rect);

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
        // ------

        // NOTE: Cannot make these constants, because some methods are not const.
        let on_black = Style::new().white().on_black();
        let on_gray = Style::new().black().on_gray();
        let on_dark_gray = Style::new().white().on_dark_gray();
        let on_light_green = Style::new().black().on_light_green();
        let on_light_red = Style::new().white().on_light_red();

        // Rendering
        // ---------

        // Render year
        // ^^^^^^^^^^^
        // NOTE: Do not seem to need to check if content is out of bounds on the horizontal axis,
        // but needs to for the vertical axis, otherwise the program will panic in attempting
        // to write outside the buffer.
        // The `Span` widget thus probably already manages attempts to write outside the bounds
        // horizontally.
        if year_rect.y < year_rect.bottom() {
            buf.set_span(
                year_rect.x,
                year_rect.y,
                &Span::styled(self.year.to_string(), on_black),
                WIDTH_FOR_YEAR,
            );
        }

        // Render day names
        // ^^^^^^^^^^^^^^^^
        if days_rect.y + 6 * HEIGHT_FOR_DAY_NAME < days_rect.bottom() {
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
        }

        // Render days matrix
        // ^^^^^^^^^^^^^^^^^^
        let mut i = 0;
        let start_x = days_mat_rect.x;
        let end_x = start_x + WIDTH_FOR_DAY * N_WEEKS_IN_YEAR;
        let start_y = days_mat_rect.y;
        let end_y = start_y + HEIGHT_FOR_DAY * N_DAYS_IN_WEEK;
        let mut month_xs = [0; 12];
        let mut month_idx = 0;
        for x in (start_x..end_x).step_by(WIDTH_FOR_DAY as usize) {
            for y in (start_y..end_y).step_by(HEIGHT_FOR_DAY as usize) {
                if y >= days_mat_rect.bottom() {
                    i += 1;
                    continue;
                }

                let span = match self.days_mat[i] {
                    Cell::NotInYear => None,
                    Cell::InYear { num, kind } => {
                        if num == 1 {
                            debug_assert!(
                                (0..month_xs.len()).contains(&month_idx),
                                "`month_idx` out of bounds; got {}",
                                month_idx
                            );
                            month_xs[month_idx] = x;
                            month_idx += 1;
                        }

                        let num_str = num.to_string();
                        let padding = (WIDTH_FOR_DAY as usize) - 1 - num_str.len();
                        let mut day_str = " ".repeat(padding);
                        day_str.push_str(&num_str);

                        match kind {
                            DayKind::ToCome => Some(Span::styled(day_str, on_dark_gray)),
                            DayKind::ShouldNotHabit => Some(Span::styled(
                                day_str,
                                if self.year == self.cur_year && i == self.today_idx {
                                    on_dark_gray
                                } else {
                                    on_gray
                                },
                            )),
                            DayKind::ShouldHabit(true) => {
                                Some(Span::styled(day_str, on_light_green))
                            }
                            DayKind::ShouldHabit(false) => {
                                Some(Span::styled(day_str, on_light_red))
                            }
                        }
                    }
                };

                if let Some(span) = span {
                    buf.set_span(x, y, &span, WIDTH_FOR_DAY - 1);
                }

                i += 1;
            }
        }

        // Render months
        // ^^^^^^^^^^^^^
        if months_rect.y < months_rect.bottom() {
            for i in 0..month_xs.len() {
                buf.set_span(
                    month_xs[i],
                    months_rect.y,
                    &Span::styled(MONTHS[i], on_black),
                    3,
                );
            }
        }

        // Render nav
        // ^^^^^^^^^^
        if nav_rect.y < nav_rect.bottom() {
            buf.set_span(
                nav_rect.x,
                nav_rect.y,
                &Span::styled("< h | o | l >", on_black),
                WIDTH_FOR_NAV,
            );
        }
    }
}
