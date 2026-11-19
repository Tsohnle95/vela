//! Locale-aware short time formatting for user-facing timestamps.
//!
//! Source formats user message timestamps with
//! `Intl.DateTimeFormat(undefined, { timeStyle: "short" })`. A localized
//! `jm` template follows the same platform locale conventions on macOS while
//! keeping a deterministic fallback elsewhere.

#[cfg(all(test, target_os = "macos"))]
pub(super) fn short_time_with_locale(
    identifier: &str,
    seconds_from_gmt: i32,
    time_ms: i64,
) -> String {
    use objc2_foundation::{NSDate, NSDateFormatter, NSLocale, NSString, NSTimeZone};
    let formatter = NSDateFormatter::new();
    formatter.setLocale(Some(&NSLocale::localeWithLocaleIdentifier(
        &NSString::from_str(identifier),
    )));
    formatter.setTimeZone(Some(&NSTimeZone::timeZoneForSecondsFromGMT(
        seconds_from_gmt as isize,
    )));
    formatter.setLocalizedDateFormatFromTemplate(&NSString::from_str("jm"));
    let date = NSDate::dateWithTimeIntervalSince1970(time_ms as f64 / 1000.);
    formatter.stringFromDate(&date).to_string()
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::short_time_with_locale;

    /// 1970-01-01 13:05:00 UTC.
    const AFTERNOON: i64 = 13 * 60 * 60 * 1000 + 5 * 60 * 1000;

    fn normalized(value: &str) -> String {
        value.replace(['\u{00A0}', '\u{202F}'], " ")
    }

    #[test]
    fn short_time_follows_the_locale_hour_cycle() {
        let us = normalized(&short_time_with_locale("en_US", 0, AFTERNOON));
        assert_eq!(us, "1:05 PM", "US short time should use a 12-hour cycle");

        let gb = normalized(&short_time_with_locale("en_GB", 0, AFTERNOON));
        assert_eq!(gb, "13:05", "UK short time should use a 24-hour cycle");
    }

    #[test]
    fn short_time_omits_seconds_like_the_source_style() {
        let us = normalized(&short_time_with_locale("en_US", 0, AFTERNOON));
        assert!(
            !us.contains(":05:00") && us.matches(':').count() == 1,
            "short time should not include seconds: {us}"
        );
    }
}
