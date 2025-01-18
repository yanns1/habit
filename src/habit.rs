use crate::utils;
use lazy_static::lazy_static;
use regex::Regex;
use std::{fmt, str::FromStr};

lazy_static! {
    static ref AT_RE: Regex = Regex::new(r"(?<hour>\d\d):(?<minutes>\d\d)").unwrap();
}

// Habit
// -----

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Habit {
    pub name: String,
    pub description: String,
    pub days: Vec<Day>,
    pub at: At,
}

impl Habit {
    pub fn new(name: String, description: String, days: Vec<Day>, at: At) -> Self {
        Self {
            name,
            description,
            days,
            at,
        }
    }
}

// At
// --

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct At {
    pub hour: u8,
    pub minutes: u8,
}

impl At {
    pub fn build(hour: u8, minutes: u8) -> Result<At, BuildAtError> {
        if hour > 23 {
            Err(BuildAtError::HourOutOfRange)
        } else if minutes > 59 {
            Err(BuildAtError::MinutesOutOfRange)
        } else {
            Ok(At { hour, minutes })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildAtError {
    HourOutOfRange,
    MinutesOutOfRange,
}

impl fmt::Display for BuildAtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HourOutOfRange => write!(f, "Hour out of range. Must be in [[0, 23]]."),
            Self::MinutesOutOfRange => write!(f, "Minutes out of range. Must be in [[0, 59]]."),
        }
    }
}

impl std::error::Error for BuildAtError {
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

impl FromStr for At {
    type Err = ParseAtError;

    fn from_str(s: &str) -> Result<Self, ParseAtError> {
        let Some(caps) = AT_RE.captures(s) else {
            return Err(ParseAtError::WrongFormat);
        };

        // Can unwrap because two digits will always be parsable into a u8.
        let hour: u8 = caps["hour"].parse().unwrap();
        let minutes: u8 = caps["minutes"].parse().unwrap();

        match At::build(hour, minutes) {
            Ok(at) => Ok(at),
            Err(BuildAtError::HourOutOfRange) => Err(ParseAtError::HourOutOfRange),
            Err(BuildAtError::MinutesOutOfRange) => Err(ParseAtError::MinutesOutOfRange),
        }
    }
}

impl fmt::Display for At {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            utils::left_pad(&self.hour.to_string(), '0', 2),
            utils::left_pad(&self.minutes.to_string(), '0', 2)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseAtError {
    WrongFormat,
    HourOutOfRange,
    MinutesOutOfRange,
}

impl fmt::Display for ParseAtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongFormat => write!(f, "Wrong format. Should match 'hh:mm'."),
            Self::HourOutOfRange => write!(f, "Hour out of range. Must be in [[0, 23]]."),
            Self::MinutesOutOfRange => write!(f, "Minutes out of range. Must be in [[0, 59]]."),
        }
    }
}

impl std::error::Error for ParseAtError {
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

// Day
// ---

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl TryFrom<u8> for Day {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Day::Monday),
            2 => Ok(Day::Tuesday),
            4 => Ok(Day::Wednesday),
            8 => Ok(Day::Thursday),
            16 => Ok(Day::Friday),
            32 => Ok(Day::Saturday),
            64 => Ok(Day::Sunday),
            _ => Err(format!(
                "Cannot get a `Day` from `u8` value given, {}",
                value
            )),
        }
    }
}

impl From<Day> for u8 {
    fn from(value: Day) -> Self {
        match value {
            Day::Monday => 1,
            Day::Tuesday => 2,
            Day::Wednesday => 4,
            Day::Thursday => 8,
            Day::Friday => 16,
            Day::Saturday => 32,
            Day::Sunday => 64,
        }
    }
}

impl From<&Day> for u8 {
    fn from(value: &Day) -> Self {
        match *value {
            Day::Monday => u8::pow(2, 0),
            Day::Tuesday => u8::pow(2, 1),
            Day::Wednesday => u8::pow(2, 2),
            Day::Thursday => u8::pow(2, 3),
            Day::Friday => u8::pow(2, 4),
            Day::Saturday => u8::pow(2, 5),
            Day::Sunday => u8::pow(2, 6),
        }
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Day::Monday => write!(f, "Monday"),
            Day::Tuesday => write!(f, "Tuesday"),
            Day::Wednesday => write!(f, "Wednesday"),
            Day::Thursday => write!(f, "Thursday"),
            Day::Friday => write!(f, "Friday"),
            Day::Saturday => write!(f, "Saturday"),
            Day::Sunday => write!(f, "Sunday"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDayError;

impl FromStr for Day {
    type Err = ParseDayError;

    fn from_str(s: &str) -> Result<Self, ParseDayError> {
        match s {
            "Monday" => Ok(Self::Monday),
            "Tuesday" => Ok(Self::Tuesday),
            "Wednesday" => Ok(Self::Wednesday),
            "Thursday" => Ok(Self::Thursday),
            "Friday" => Ok(Self::Friday),
            "Saturday" => Ok(Self::Saturday),
            "Sunday" => Ok(Self::Sunday),
            _ => Err(ParseDayError),
        }
    }
}

pub fn days_to_byte(days: &[Day]) -> u8 {
    days.iter()
        .fold(0, |acc, d| acc + <&Day as Into<u8>>::into(d))
}

pub fn byte_to_days(mut byte: u8) -> Vec<Day> {
    let mut days = vec![];
    if byte & 1 == 1 {
        days.push(Day::Monday);
    }
    byte >>= 1;
    if byte & 1 == 1 {
        days.push(Day::Tuesday);
    }
    byte >>= 1;
    if byte & 1 == 1 {
        days.push(Day::Wednesday);
    }
    byte >>= 1;
    if byte & 1 == 1 {
        days.push(Day::Thursday);
    }
    byte >>= 1;
    if byte & 1 == 1 {
        days.push(Day::Friday);
    }
    byte >>= 1;
    if byte & 1 == 1 {
        days.push(Day::Saturday);
    }
    byte >>= 1;
    if byte & 1 == 1 {
        days.push(Day::Sunday);
    }

    days
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use std::collections::HashSet;

    #[test]
    fn conversion_between_days_and_byte() {
        let mut days = vec![Day::Monday, Day::Tuesday];
        let n_days = days.len();
        for perm in days.into_iter().permutations(n_days) {
            let byte = days_to_byte(&perm[..]);
            let days = byte_to_days(byte);
            let perm_set: HashSet<Day> = perm.into_iter().collect();
            let days_set: HashSet<Day> = days.into_iter().collect();
            assert_eq!(days_set, perm_set);
        }
    }
}
