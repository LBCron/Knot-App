//! Watching a terminal agent's output for its usage-limit notice (#36).
//!
//! A Panel-mode agent learns of the limit from the error that answers its
//! prompt (#318). A terminal agent prints it instead - Claude Code's
//! "You've hit your session limit · resets 5:20pm (America/Los_Angeles)",
//! or "Usage limit reached · wrapping up" when it is first allowed to finish
//! what it was doing - and this is where Knot reads it.
//!
//! Like the pull request scan beside it, this reads the PTY byte stream,
//! not the rendered grid: every byte once, on the reader thread. Escape
//! sequences are dropped as they arrive, a cursor move ends a line the way a
//! newline does, and only a line's start is compared - one short prefix
//! check per line, with nothing buffered past [`SESSION_LIMIT_MAX_LINE`].

use std::time::Instant;

use knot_agents::SessionLimit;

use crate::consts::{SESSION_LIMIT_MAX_LINE, SESSION_LIMIT_NOTICES, SESSION_LIMIT_REPEAT};

/// Where the scan is inside an escape sequence, carried across chunks
/// because a PTY read can split one anywhere.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Escape {
    #[default]
    None,
    /// After ESC.
    Esc,
    /// Inside `ESC [`, until its final byte.
    Csi,
    /// Inside `ESC ]`, until BEL or `ESC \`.
    Osc,
    /// ESC inside an OSC: the start of its `ESC \` terminator.
    OscEsc,
}

/// The scan and what it has found, fed on the PTY reader thread and taken
/// by the window's poll.
#[derive(Debug, Default)]
pub(crate) struct SessionLimitTap {
    escape:   Escape,
    /// The current line, printable bytes only.
    line:     Vec<u8>,
    /// The current line outgrew [`SESSION_LIMIT_MAX_LINE`]: not a notice.
    overflow: bool,
    /// The limit last reported, and when its notice was last seen.
    last:     Option<(SessionLimit, Instant)>,
    /// Found but not yet handed to the window.
    pending:  Option<SessionLimit>,
}

impl SessionLimitTap {
    /// Scan a chunk of PTY output read at `now`.
    pub(crate) fn feed(&mut self, bytes: &[u8], now: Instant) {
        for &byte in bytes {
            self.step(byte, now);
        }
        // The notice can be the last line drawn, with no newline after it
        // until the next frame.
        self.check_line(now);
    }

    /// Take the limit seen since the last call, if any.
    pub(crate) fn take(&mut self) -> Option<SessionLimit> {
        self.pending.take()
    }

    fn step(&mut self, byte: u8, now: Instant) {
        self.escape = match (self.escape, byte) {
            (Escape::None, 0x1B) => Escape::Esc,
            (Escape::None, b'\n' | b'\r') => {
                self.end_line(now);
                Escape::None
            }
            (Escape::None, byte) => {
                if byte >= 0x20 || byte == b'\t' {
                    self.push(byte);
                }
                Escape::None
            }
            (Escape::Esc, b'[') => Escape::Csi,
            (Escape::Esc, b']') => Escape::Osc,
            (Escape::Esc, _) => Escape::None,
            (Escape::Csi, 0x40..=0x7E) => {
                match byte {
                    // Cursor forward: a gap in the same line.
                    b'C' => self.push(b' '),
                    // Any other cursor move leaves the line.
                    b'A' | b'B' | b'E' | b'F' | b'G' | b'H' | b'd' | b'f' => self.end_line(now),
                    _ => {}
                }
                Escape::None
            }
            (Escape::Csi, _) => Escape::Csi,
            (Escape::Osc, 0x07) => Escape::None,
            (Escape::Osc, 0x1B) => Escape::OscEsc,
            (Escape::Osc, _) => Escape::Osc,
            (Escape::OscEsc, b'\\') => Escape::None,
            (Escape::OscEsc, _) => Escape::Osc,
        };
    }

    fn push(&mut self, byte: u8) {
        if self.line.len() < SESSION_LIMIT_MAX_LINE {
            self.line.push(byte);
        }
        else {
            self.overflow = true;
        }
    }

    fn end_line(&mut self, now: Instant) {
        self.check_line(now);
        self.line.clear();
        self.overflow = false;
    }

    fn check_line(&mut self, now: Instant) {
        if self.overflow || self.line.is_empty() {
            return;
        }
        let Some(limit) = notice(&String::from_utf8_lossy(&self.line))
        else {
            return;
        };
        let repeat =
            self.last
                .as_ref()
                .is_some_and(|(last, seen)| {
                    *last == limit && now.saturating_duration_since(*seen) < SESSION_LIMIT_REPEAT
                });
        self.last = Some((limit.clone(), now));
        if !repeat {
            self.pending = Some(limit);
        }
    }
}

/// The limit `line` announces, if it opens with a notice. Leading
/// decoration - a bullet, a box edge, indentation - is skipped first.
fn notice(line: &str) -> Option<SessionLimit> {
    let text = line.trim_start_matches(|c: char| !c.is_alphanumeric())
                   .trim_end();
    let lower = text.to_lowercase().replace('\u{2019}', "'");
    if !SESSION_LIMIT_NOTICES.iter()
                             .any(|notice| lower.starts_with(notice))
    {
        return None;
    }
    Some(SessionLimit::from_error(text).unwrap_or(SessionLimit { resets: None }))
}

#[cfg(test)]
mod tests;
