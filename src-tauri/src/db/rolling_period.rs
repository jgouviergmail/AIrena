//! Rolling monthly period arithmetic shared by every usage counter
//! (Tavily credits, DeepSeek spend). Pure functions — persistence stays in
//! `repository.rs`.

use chrono::{Datelike, NaiveDate};

/// Result of checking a period against today's date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rollover {
    /// End (exclusive) of the period that just expired.
    pub expired_end: NaiveDate,
    /// Start of the period containing `today`.
    pub new_start: NaiveDate,
}

/// Parse a stored `YYYY-MM-DD` period start.
pub fn parse_period_start(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

/// End (exclusive) of the period starting at `start`: one calendar month later.
pub fn period_end(start: NaiveDate) -> NaiveDate {
    add_one_month(start)
}

/// If `today` is past the end of the period starting at `start`, return the
/// rollover with the new start advanced by whole months so that it contains `today`.
pub fn rollover(start: NaiveDate, today: NaiveDate) -> Option<Rollover> {
    let expired_end = period_end(start);
    if today < expired_end {
        return None;
    }
    let mut new_start = expired_end;
    while add_one_month(new_start) <= today {
        new_start = add_one_month(new_start);
    }
    Some(Rollover { expired_end, new_start })
}

/// Add one calendar month, clamping to the last day of the target month.
pub fn add_one_month(date: NaiveDate) -> NaiveDate {
    let (year, month) = if date.month() == 12 {
        (date.year() + 1, 1)
    } else {
        (date.year(), date.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, date.day()).unwrap_or_else(|| {
        let last_day = last_day_of_month(year, month);
        NaiveDate::from_ymd_opt(year, month, last_day).expect("valid clamped date")
    })
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(28)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn add_one_month_clamps_to_month_end() {
        assert_eq!(add_one_month(d(2026, 1, 31)), d(2026, 2, 28));
        assert_eq!(add_one_month(d(2028, 1, 31)), d(2028, 2, 29)); // leap year
        assert_eq!(add_one_month(d(2026, 3, 31)), d(2026, 4, 30));
        assert_eq!(add_one_month(d(2026, 12, 15)), d(2027, 1, 15));
    }

    #[test]
    fn rollover_none_inside_period() {
        assert!(rollover(d(2026, 9, 1), d(2026, 9, 30)).is_none());
        assert!(rollover(d(2026, 9, 1), d(2026, 9, 1)).is_none());
    }

    #[test]
    fn rollover_on_boundary_and_after_several_months() {
        let r = rollover(d(2026, 9, 1), d(2026, 10, 1)).unwrap();
        assert_eq!(r.expired_end, d(2026, 10, 1));
        assert_eq!(r.new_start, d(2026, 10, 1));

        // Three months of inactivity: new start lands in the month containing today
        let r = rollover(d(2026, 6, 10), d(2026, 9, 16)).unwrap();
        assert_eq!(r.expired_end, d(2026, 7, 10));
        assert_eq!(r.new_start, d(2026, 9, 10));
    }

    #[test]
    fn parse_period_start_tolerates_whitespace_and_rejects_garbage() {
        assert_eq!(parse_period_start(" 2026-09-16 "), Some(d(2026, 9, 16)));
        assert!(parse_period_start("16/09/2026").is_none());
        assert!(parse_period_start("").is_none());
    }
}
