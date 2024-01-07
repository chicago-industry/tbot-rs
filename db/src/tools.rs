use chrono::{Duration, NaiveDate, NaiveTime, Utc};

pub fn datetime_utc3() -> (NaiveDate, NaiveTime) {
    let d = Utc::now() + Duration::hours(3);
    (d.date_naive(), d.time())
}

// if the transmitted date is greater than the current one,
// then you need to make a sample for the entire day, i.e. from 00:00 hours
pub(super) fn time_determine(date: NaiveDate) -> NaiveTime {
    let (curr_date, curr_time) = datetime_utc3();

    if date > curr_date {
        NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    } else {
        curr_time
    }
}
