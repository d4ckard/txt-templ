use super::Content;
use chrono::Utc;
use lazy_static::lazy_static;

pub enum Meta {
    /// The current year (e.g. 2023)
    Year,
    /// The number of the current month where January is 01 and December is 12.
    MonthNum,
    /// The full name of the current month.
    MonthName,
    /// The name of the current month abbreviated to three letters.
    MonthAbbr,
    /// The number of the current day in the month (01-31).
    DayNum,
    /// The number of the current week in the year.
    Week,
    /// The name of the current day.
    DayName,
    /// The name of the current day abbreviated to three letters.
    DayAbbr,
    /// The current hour of the day where the first hour is 00 and the last hour is 23.
    Hour,
    /// The current minute of the hour where the first minute is 00 and the last minute is 59.
    Minute,
    /// The current second of the minute where the first second is 00 and the last second is 59.
    Second,
}

impl From<Meta> for Content {
    /// Get the value of the meta element as a string.
    fn from(from: Meta) -> Self {
        let time_date_fmt = |s| Utc::now().format(s).to_string();

        match from {
            Meta::Year => time_date_fmt("%Y"),
            Meta::MonthNum => time_date_fmt("%m"),
            Meta::MonthName => time_date_fmt("%B"),
            Meta::MonthAbbr => time_date_fmt("%b"),
            Meta::DayNum => time_date_fmt("%d"),
            Meta::Week => time_date_fmt("%U"),
            Meta::DayName => time_date_fmt("%A"),
            Meta::DayAbbr => time_date_fmt("%a"),
            Meta::Hour => time_date_fmt("%H"),
            Meta::Minute => time_date_fmt("%M"),
            Meta::Second => time_date_fmt("%S"),
        }
    }
}

pub trait MetaExt {
    /// Convert `self` into the matching `Meta` instance if `self` has a meta value.
    fn as_meta(&self) -> Option<Meta>;
}

impl<S> MetaExt for S
where
    S: AsRef<str>,
{
    fn as_meta(&self) -> Option<Meta> {
        Some(match self.as_ref() {
            "Year" => Meta::Year,
            "MonthNum" => Meta::MonthNum,
            "Month" | "MonthName" => Meta::MonthName,
            "MonthAbbr" | "MonthShort" => Meta::MonthAbbr,
            "DayNum" => Meta::DayNum,
            "Week" => Meta::Week,
            "Day" | "DayName" => Meta::DayName,
            "DayAbbr" | "DayShort" => Meta::DayAbbr,
            "Hour" => Meta::Hour,
            "Minute" => Meta::Minute,
            "Second" => Meta::Second,
            _ => {
                return None;
            }
        })
    }
}

// Lists of the identifiers of all meta elements.
pub const META_DATE_TIME: [&str; 5] = ["Year", "Week", "Hour", "Minute", "Second"];
pub const META_MONTH: [&str; 5] = ["MonthNum", "Month", "MonthName", "MonthAbbr", "MonthShort"];
pub const META_DAY: [&str; 5] = ["DayNum", "Day", "DayName", "DayAbbr", "DayShort"];
// List of all meta identifiers.
lazy_static! {
    static ref META_IDENTS: Vec<&'static str> = [META_DATE_TIME, META_MONTH, META_DAY].concat();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Ident;

    #[test]
    fn idents_can_become_meta() {
        for ident in META_IDENTS.iter() {
            assert!(Ident::from(*ident).as_meta().is_some());
        }
    }

    #[test]
    fn date_time_meta_is_correct() {
        // The string behind each meta value will be compared to this time value.
        // It suffices to get the current time once, because the time between the
        // initial asserts and is much less than a second, so it will not become outdated.
        let now = Utc::now();

        // Check that the given meta value becomes a time/date-string with the given formatting.
        let check_meta_to_dt = |meta, fmt| {
            let meta_str = Content::from(meta);
            let dt_str = now.format(fmt).to_string();

            assert_eq!(meta_str, dt_str);
        };

        check_meta_to_dt(Meta::Second, "%S");
        check_meta_to_dt(Meta::Minute, "%M");
        check_meta_to_dt(Meta::Hour, "%H");
        check_meta_to_dt(Meta::DayNum, "%d");
        check_meta_to_dt(Meta::DayName, "%A");
        check_meta_to_dt(Meta::DayAbbr, "%a");
        check_meta_to_dt(Meta::Week, "%U");
        check_meta_to_dt(Meta::MonthNum, "%m");
        check_meta_to_dt(Meta::MonthName, "%B");
        check_meta_to_dt(Meta::MonthAbbr, "%b");
    }

    // Test cases asserting that meta elements work as expected.

    use crate::content::{UserContent, UserContentState};
    use crate::template::helper::test_fill_out;

    #[test]
    fn meta_constants_are_recognised_and_evaluated() {
        for meta_ident in META_IDENTS.iter() {
            let meta_constant = &format!("${}", *meta_ident);
            let expected = &Content::from(meta_ident.as_meta().unwrap());
            test_fill_out(
                meta_constant,
                expected,
                &format!("Meta constant {meta_constant} evaluation"),
                UserContent::new(),
                UserContentState::new(),
            );
        }
    }
}
