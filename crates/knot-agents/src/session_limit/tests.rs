//! Recognising a usage limit from the error that answers a prompt (#318).

use super::SessionLimit;

/// The error from the issue's screenshot, verbatim.
#[test]
fn claudes_session_limit_is_recognised_with_its_reset_time() {
    let error = "The agent could not answer: acp error response (code -32603): Internal \
                 error: You've hit your session limit · resets 5:20pm (America/Los_Angeles)";

    assert_eq!(SessionLimit::from_error(error),
               Some(SessionLimit { resets: Some("5:20pm (America/Los_Angeles)".to_owned()), }));
}

#[test]
fn a_usage_limit_without_a_reset_time_is_still_a_limit() {
    assert_eq!(SessionLimit::from_error("You've hit your usage limit."),
               Some(SessionLimit { resets: None }));
}

#[test]
fn any_other_error_is_not_a_limit() {
    for error in ["quota exhausted",
                  "acp error response (code -32603): Internal error",
                  "connection reset by peer"]
    {
        assert_eq!(SessionLimit::from_error(error), None, "{error}");
    }
}

/// 2026-11-04 12:00 UTC: 04:00 in Los Angeles, 13:00 in Paris.
fn refused_at() -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_793_793_600)
}

fn limit(resets: &str) -> SessionLimit {
    SessionLimit { resets: Some(resets.to_owned()), }
}

fn utc(seconds: u64) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds)
}

/// #368: the reset is the next time the named zone's clock reads it.
#[test]
fn a_reset_later_the_same_day_lifts_that_day() {
    // 17:20 in Los Angeles (PST, UTC-8) on 2026-11-04 is 01:20 UTC on the 5th.
    assert_eq!(limit("5:20pm (America/Los_Angeles)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 13 * 3600 + 20 * 60)));
}

#[test]
fn a_reset_already_past_today_lifts_tomorrow() {
    // 09:00 in Paris has passed at 13:00 Paris time, so it is 09:00 on the
    // 5th: 08:00 UTC (CET, UTC+1).
    assert_eq!(limit("9am (Europe/Paris)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 20 * 3600)));
}

#[test]
fn twelve_oclock_is_noon_or_midnight() {
    // 12pm in Paris is 11:00 UTC on the 5th (passed today); 12am is 23:00
    // UTC on the 4th.
    assert_eq!(limit("12pm (Europe/Paris)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 23 * 3600)));
    assert_eq!(limit("12am (Europe/Paris)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 11 * 3600)));
}

/// A time this cannot be sure of is no time to send a held prompt at.
#[test]
fn a_reset_it_cannot_read_has_no_time() {
    for resets in ["in 3 hours",
                   "5:20pm",
                   "5:20pm (Not/AZone)",
                   "Nov 5, 9am",
                   "25pm (Europe/Paris)",
                   "5:75pm (Europe/Paris)"]
    {
        assert_eq!(limit(resets).lifts_at(refused_at()), None, "{resets}");
    }
    assert_eq!(SessionLimit { resets: None }.lifts_at(refused_at()), None);
}
