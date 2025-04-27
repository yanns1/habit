use crate::TODAY;
use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use chrono::MappedLocalTime;
use chrono::TimeZone;
use clap::Args;
use regex::Regex;
use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

pub static PAST_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?<year>\d\d\d\d)-(?<month>\d\d)-(?<day>\d\d)\s*((?<hour>\d\d):(?<minutes>\d\d))?\s*$").unwrap()
});

#[derive(Args, Debug, Clone, PartialEq, Eq)]
#[clap(verbatim_doc_comment)]
/// Log a rep for a habit.
pub struct LogCli {
    #[clap(verbatim_doc_comment)]
    /// The name of the habit for which to log a rep.
    pub habit: String,

    #[clap(verbatim_doc_comment)]
    #[arg(value_parser = PastDate::from_str)]
    /// A past date for which to log, in case you forgot to.
    pub date: Option<PastDate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PastDate {
    Last,
    Date(DateTime<Local>),
}

impl FromStr for PastDate {
    type Err = ParsePastDateError;

    fn from_str(s: &str) -> Result<Self, ParsePastDateError> {
        if s == "last" {
            return Ok(PastDate::Last);
        }

        let Some(caps) = PAST_DATE_RE.captures(s) else {
            return Err(ParsePastDateError::WrongFormat);
        };

        let year: i32 = caps
            .name("year")
            .expect("Capture group named 'year' should be present.")
            .as_str()
            .parse()
            .unwrap();
        let month: u32 = caps
            .name("month")
            .expect("Capture group named 'month' should be present.")
            .as_str()
            .parse()
            .unwrap();
        let day: u32 = caps
            .name("day")
            .expect("Capture group named 'day' should be present.")
            .as_str()
            .parse()
            .unwrap();
        let (hour, minutes) = if let Some(hour_match) = caps.name("hour") {
            let hour: u32 = hour_match.as_str().parse().unwrap();
            let minutes: u32 = caps
                .name("minutes")
                .expect("Capture group named 'minutes' should be present.")
                .as_str()
                .parse()
                .unwrap();
            (hour, minutes)
        } else {
            (0, 0)
        };

        // Check if valid date.
        let dt_res = Local.with_ymd_and_hms(year, month, day, hour, minutes, 0);
        if dt_res == MappedLocalTime::None {
            return Err(ParsePastDateError::InvalidDate);
        }
        let dt = dt_res.unwrap();

        // Check if past date.
        let first_second_of_today = Local
            .with_ymd_and_hms(TODAY.year(), TODAY.month(), TODAY.day(), 0, 0, 0)
            .unwrap();
        if dt >= first_second_of_today {
            return Err(ParsePastDateError::NotInThePast);
        }

        Ok(PastDate::Date(dt))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsePastDateError {
    WrongFormat,
    InvalidDate,
    NotInThePast,
}

impl fmt::Display for ParsePastDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongFormat => write!(
                f,
                "Wrong format. Should match 'last', 'yyyy-mm-dd' or 'yyyy-mm-dd hh:mm'."
            ),
            Self::InvalidDate => write!(
                f,
                "Invalid date. Check that your date is in the Gregorian calendar."
            ),
            Self::NotInThePast => write!(f, "The date is not in the past."),
        }
    }
}

impl std::error::Error for ParsePastDateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}
