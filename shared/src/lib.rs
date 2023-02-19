use std::{fmt::Display, ops::RangeInclusive, str::FromStr};

use anyhow::anyhow;
pub use chrono::{Datelike, Month};
use num_traits::FromPrimitive;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct YearMonth {
    pub year: i32,
    pub month: Month,
}

impl YearMonth {
    pub fn iter_range(range: &RangeInclusive<Self>) -> impl Iterator<Item = Self> + '_ {
        range.start().take_while(|month| *month <= *range.end())
    }
}

impl PartialOrd for YearMonth {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let same_year = self.year.partial_cmp(&other.year)?;
        let same_month = self
            .month
            .number_from_month()
            .partial_cmp(&other.month.number_from_month())?;
        Some(same_year.then(same_month))
    }
}

impl Ord for YearMonth {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.year.cmp(&other.year).then(
            self.month
                .number_from_month()
                .cmp(&other.month.number_from_month()),
        )
    }
}

impl Iterator for YearMonth {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        let this = *self;
        self.month = this.month.succ();
        if self.month.number_from_month() < this.month.number_from_month() {
            self.year += 1;
        }
        Some(this)
    }
}

impl Display for YearMonth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{:02}", self.year, self.month.number_from_month())
    }
}

impl FromStr for YearMonth {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"(\d{4})-(\d{2})")?;
        let cap = re.captures(s).ok_or_else(|| {
            anyhow!("Invalid year/month format. The correct form is YYYY-MM: {s}")
        })?;
        Ok(YearMonth {
            year: cap[1].parse()?,
            month: Month::from_u64(cap[2].parse::<u64>()?)
                .ok_or_else(|| anyhow!("Invalid month: {}", &cap[2]))?,
        })
    }
}

/// Messages from the backend to the frontend
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    /// Standard output
    Message(String),
    /// Error output
    Error(String),
    /// Download completion message
    Complete(YearMonth),
}

impl Message {
    pub fn message(message: String) -> Self {
        Self::Message(message)
    }
    pub fn error(message: String) -> Self {
        Self::Error(message)
    }
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

impl From<YearMonth> for Message {
    fn from(month: YearMonth) -> Self {
        Self::Complete(month)
    }
}

impl From<anyhow::Error> for Message {
    fn from(err: anyhow::Error) -> Self {
        Self::Error(format!("{err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_year_month() {
        assert_eq!(
            YearMonth::from_str("2020-01").unwrap(),
            YearMonth {
                year: 2020,
                month: Month::January
            }
        )
    }

    #[test]
    fn invalid_year_month() {
        assert!(YearMonth::from_str("2020-13").is_err())
    }

    #[test]
    fn year_month_partial_cmp() {
        let dec_2020 = YearMonth {
            year: 2020,
            month: Month::December,
        };
        let jan_2021 = YearMonth {
            year: 2021,
            month: Month::January,
        };
        let feb_2021 = YearMonth {
            year: 2021,
            month: Month::February,
        };
        assert!(dec_2020 < jan_2021);
        assert!(jan_2021 < feb_2021);
    }

    #[test]
    fn year_month_iter() {
        let dec_2020 = YearMonth {
            year: 2020,
            month: Month::December,
        };
        assert_eq!(
            dec_2020.take(3).collect::<Vec<_>>(),
            vec![
                YearMonth {
                    year: 2020,
                    month: Month::December
                },
                YearMonth {
                    year: 2021,
                    month: Month::January
                },
                YearMonth {
                    year: 2021,
                    month: Month::February
                }
            ]
        )
    }
}
