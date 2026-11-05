//! How much of its usage allowance Codex has used, read from the session
//! files it writes (#291).
//!
//! Neither ACP adapter forwards usage today (`claude-agent-acp` #625,
//! `codex-acp` #227 are open). Codex records it anyway: every
//! `token_count` event in its `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
//! carries a `rate_limits` snapshot - the 5-hour and weekly windows' used
//! percentage and reset time, from OpenAI's response headers. The figures
//! are per account, so the newest snapshot in the newest file is the
//! current one, whichever agent wrote it.
//!
//! Read-only and forgiving: a missing directory, an unreadable file or a
//! format that has changed all come back as nothing to show, never an
//! error. This is advice, and Codex owns the format.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::consts::{CODEX_USAGE_NEAR_LIMIT_PERCENT, CODEX_USAGE_TAIL_BYTES};

/// One usage window: how much of it is used, and when it starts afresh.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UsageWindow {
    pub(crate) used_percent: f64,
    pub(crate) resets_at:    Option<SystemTime>,
}

/// The windows in Codex's latest rate-limit snapshot.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CodexUsage {
    pub(crate) windows: Vec<UsageWindow>,
}

impl CodexUsage {
    /// The window nearest its limit, if one is at or past
    /// [`CODEX_USAGE_NEAR_LIMIT_PERCENT`] and has not reset since the
    /// snapshot was taken. A window whose reset is past says nothing about
    /// now: it has started over.
    pub(crate) fn near_limit(&self, now: SystemTime) -> Option<&UsageWindow> {
        self.windows
            .iter()
            .filter(|window| window.used_percent >= CODEX_USAGE_NEAR_LIMIT_PERCENT)
            .filter(|window| window.resets_at.is_none_or(|resets| resets > now))
            .max_by(|a, b| a.used_percent.total_cmp(&b.used_percent))
    }
}

/// Codex's current usage, read from the newest session file under
/// `$CODEX_HOME` (else `~/.codex`). `None` when there is nothing to read.
pub(crate) fn read_current() -> Option<CodexUsage> {
    let home =
        std::env::var_os("CODEX_HOME").map(PathBuf::from)
                                      .or_else(|| {
                                          std::env::home_dir().map(|home| home.join(".codex"))
                                      })?;
    read_from(&home)
}

/// [`read_current`] for a given Codex home.
pub(crate) fn read_from(codex_home: &Path) -> Option<CodexUsage> {
    let rollout = newest_rollout(&codex_home.join("sessions"))?;
    parse_snapshot(&read_tail(&rollout)?)
}

/// The newest `rollout-*.jsonl` under `sessions`, by modification time,
/// looking in the newest dated directory that has any.
fn newest_rollout(sessions: &Path) -> Option<PathBuf> {
    for year in newest_first(sessions) {
        for month in newest_first(&year) {
            for day in newest_first(&month) {
                let newest =
                    std::fs::read_dir(&day).ok()?
                                           .filter_map(Result::ok)
                                           .filter(|entry| {
                                               entry.file_name()
                                                    .to_string_lossy()
                                                    .starts_with("rollout-")
                                           })
                                           .filter_map(|entry| {
                                               let modified =
                                                   entry.metadata().ok()?.modified().ok()?;
                                               Some((modified, entry.path()))
                                           })
                                           .max_by_key(|(modified, _)| *modified);
                if let Some((_, path)) = newest {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// `dir`'s numeric subdirectories - `2026`, `09`, `25` - newest first.
fn newest_first(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir)
    else {
        return Vec::new();
    };
    let mut dated = entries.filter_map(Result::ok)
                           .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                           .filter_map(|entry| {
                               let number = entry.file_name().to_str()?.parse::<u32>().ok()?;
                               Some((number, entry.path()))
                           })
                           .collect::<Vec<_>>();
    dated.sort_by_key(|(number, _)| std::cmp::Reverse(*number));
    dated.into_iter().map(|(_, path)| path).collect()
}

/// The end of `path`: the newest events are last, and a long session's file
/// runs to megabytes. A line cut by the start of the window is dropped.
fn read_tail(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    let start = length.saturating_sub(CODEX_USAGE_TAIL_BYTES);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if start == 0 {
        return Some(text);
    }
    text.split_once('\n').map(|(_, rest)| rest.to_owned())
}

/// The newest rate-limit snapshot in rollout `text`: the last `token_count`
/// event whose `rate_limits` has a window.
pub(crate) fn parse_snapshot(text: &str) -> Option<CodexUsage> {
    text.lines().rev().find_map(|line| {
                          let event: Value = serde_json::from_str(line).ok()?;
                          let payload = event.get("payload")?;
                          if payload.get("type")?.as_str()? != "token_count" {
                              return None;
                          }
                          let limits = payload.get("rate_limits")?;
                          let windows =
                              ["primary", "secondary"].iter()
                                                      .filter_map(|name| window(limits.get(name)?))
                                                      .collect::<Vec<_>>();
                          (!windows.is_empty()).then_some(CodexUsage { windows })
                      })
}

fn window(value: &Value) -> Option<UsageWindow> {
    let used_percent = value.get("used_percent")?.as_f64()?;
    let resets_at = value.get("resets_at")
                         .and_then(Value::as_u64)
                         .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds));
    Some(UsageWindow { used_percent,
                       resets_at })
}

#[cfg(test)]
mod tests;
