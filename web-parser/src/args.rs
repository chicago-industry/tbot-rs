use super::URL_MOSKINO_SCHEDULE;

use chrono::{Duration, NaiveDate, Utc};
use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
pub(super) struct Args {
    // what day to parse movies
    #[arg(short, long, default_value = "today")]
    #[clap(value_enum)]
    pub day: ArgDay,
}

#[derive(Debug, ValueEnum, Clone)]
pub(super) enum ArgDay {
    Today,
    Tommorow,
    Aftertommorow,
}

impl ArgDay {
    pub(super) fn date(&self) -> NaiveDate {
        match self {
            Self::Today => (Utc::now() + Duration::hours(3)).date_naive(),
            Self::Tommorow => (Utc::now() + Duration::hours(3) + Duration::days(1)).date_naive(),
            Self::Aftertommorow => (Utc::now() + Duration::hours(3) + Duration::days(2)).date_naive(),
        }
    }

    pub(super) fn url_by_day(&self) -> String {
        format!("{}?date={}", URL_MOSKINO_SCHEDULE, self.date().format("%Y-%m-%d"))
    }
}
