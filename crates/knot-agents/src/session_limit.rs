//! A session stopped by its provider's usage limit (#318).
//!
//! The agent CLIs report it as the error that answers a prompt - Claude's
//! "You've hit your session limit · resets 5:20pm (America/Los_Angeles)",
//! Codex's "You've hit your usage limit" - so it is recognised from that
//! text. Nothing else distinguishes it from any other failed turn, and it is
//! the one failure the user can do nothing about but wait.

use std::time::SystemTime;

/// Phrases, lower-case, that mark an error as a usage limit.
const LIMIT_PHRASES: [&str; 2] = ["session limit", "usage limit"];

/// Where the reset time starts, lower-case.
const RESETS: &str = "resets ";

/// Hours on a 12-hour clock.
const HALF_DAY: i8 = 12;

/// An agent's session has hit its usage limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLimit {
    /// When it lifts, as the provider said it - "5:20pm
    /// (America/Los_Angeles)" - or `None` when it did not say.
    pub resets: Option<String>,
}

impl SessionLimit {
    /// The limit `error` reports, if it reports one.
    #[must_use]
    pub fn from_error(error: &str) -> Option<Self> {
        // ASCII-only lowering keeps every byte offset where it was, so an
        // index found in `lower` slices `error` at the same place.
        let lower = error.to_ascii_lowercase();
        if !LIMIT_PHRASES.iter().any(|phrase| lower.contains(phrase)) {
            return None;
        }
        let resets = lower.find(RESETS)
                          .map(|at| error[at + RESETS.len()..].trim().trim_end_matches('.'))
                          .filter(|resets| !resets.is_empty())
                          .map(str::to_owned);
        Some(Self { resets })
    }

    /// When the limit lifts, for one reported at `refused_at` (#368): the
    /// next time the clock in [`Self::resets`] reads its time of day, in the
    /// zone it names - "5:20pm (America/Los_Angeles)", "9am (Europe/Paris)".
    ///
    /// `None` when there is no reset time, or one in a shape this does not
    /// read (a date, a duration) or naming a zone the system does not know:
    /// a time it is not sure of is no time to send a held prompt at.
    #[must_use]
    pub fn lifts_at(&self, refused_at: SystemTime) -> Option<SystemTime> {
        let (clock, zone) = self.resets.as_deref()?.split_once('(')?;
        let zone = zone.trim().strip_suffix(')')?.trim();
        let (hour, minute) = parse_clock(clock.trim())?;
        let zone = jiff::tz::TimeZone::get(zone).ok()?;
        let refused = jiff::Timestamp::try_from(refused_at).ok()?
                                                           .to_zoned(zone.clone());
        let same_day = refused.date()
                              .at(hour, minute, 0, 0)
                              .to_zoned(zone.clone())
                              .ok()?;
        let lifts = if same_day.timestamp() > refused.timestamp() {
            same_day
        }
        else {
            refused.date()
                   .tomorrow()
                   .ok()?
                   .at(hour, minute, 0, 0)
                   .to_zoned(zone)
                   .ok()?
        };
        Some(lifts.timestamp().into())
    }
}

/// A 12-hour clock time - "5:20pm", "9am", "12:05 AM" - as a 24-hour hour
/// and minute.
fn parse_clock(clock: &str) -> Option<(i8, i8)> {
    let lower = clock.to_ascii_lowercase();
    let (digits, afternoon) = if let Some(digits) = lower.strip_suffix("pm") {
        (digits, true)
    }
    else {
        (lower.strip_suffix("am")?, false)
    };
    let (hour, minute) = match digits.trim().split_once(':') {
        Some((hour, minute)) => (hour.parse::<i8>().ok()?, minute.parse::<i8>().ok()?),
        None => (digits.trim().parse::<i8>().ok()?, 0),
    };
    if !(1..=HALF_DAY).contains(&hour) || !(0..60).contains(&minute) {
        return None;
    }
    let hour = match (hour, afternoon) {
        (HALF_DAY, false) => 0,
        (HALF_DAY, true) => HALF_DAY,
        (hour, true) => hour + HALF_DAY,
        (hour, false) => hour,
    };
    Some((hour, minute))
}

#[cfg(test)]
mod tests;
