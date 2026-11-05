//! Reading Codex's usage from its session files (#291), against the event
//! shape Codex writes - captured from a real `rollout-*.jsonl`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tempfile::TempDir;

use super::*;

/// A `token_count` event as Codex writes it, with its windows filled in.
fn token_count(primary: &str, secondary: &str) -> String {
    format!(r#"{{"timestamp":"2026-09-25T16:50:36.933Z","ordinal":37,"type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":72034,"output_tokens":707,"total_tokens":72741}},"model_context_window":258400}},"rate_limits":{{"limit_id":"codex","limit_name":null,"primary":{primary},"secondary":{secondary},"credits":{{"has_credits":false,"unlimited":false,"balance":null}},"plan_type":"plus","rate_limit_reached_type":null}}}}}}"#)
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn the_newest_snapshot_is_read_with_both_windows() {
    let text = [token_count(r#"{"used_percent":10.0,"window_minutes":300,"resets_at":1790000000}"#,
                            "null"),
                r#"{"type":"event_msg","payload":{"type":"agent_message","message":"hi"}}"#.to_owned(),
                token_count(r#"{"used_percent":82.0,"window_minutes":300,"resets_at":1790000000}"#,
                            r#"{"used_percent":40.0,"window_minutes":10080,"resets_at":1790500000}"#)].join("\n");

    let usage = parse_snapshot(&text).expect("a snapshot");

    assert_eq!(usage.windows,
               vec![UsageWindow { used_percent: 82.0,
                                  resets_at:    Some(at(1_790_000_000)), },
                    UsageWindow { used_percent: 40.0,
                                  resets_at:    Some(at(1_790_500_000)), }]);
}

/// A field Codex leaves out, or nulls, is no reason to fail the rest.
#[test]
fn a_null_window_or_a_missing_reset_is_tolerated() {
    let usage =
        parse_snapshot(&token_count(r#"{"used_percent":90.5}"#, "null")).expect("a snapshot");

    assert_eq!(usage.windows,
               vec![UsageWindow { used_percent: 90.5,
                                  resets_at:    None, }]);
}

/// A format Codex has changed, or lines that are not events, read as
/// nothing to show - never as an error.
#[test]
fn an_unknown_format_reads_as_nothing() {
    for text in ["",
                 "not json",
                 r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":null}}"#,
                 r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":null,"secondary":null}}}"#,
                 r#"{"payload":{"type":"token_count","rate_limits":{"primary":{"used":"a lot"}}}}"#]
    {
        assert_eq!(parse_snapshot(text), None, "{text:?}");
    }
}

#[test]
fn only_a_window_near_its_limit_and_not_yet_reset_is_near_the_limit() {
    let now = at(1_790_000_000);
    let usage = CodexUsage { windows: vec![UsageWindow { used_percent: 60.0,
                                                         resets_at:    Some(at(1_790_003_600)), },
                                           UsageWindow { used_percent: 80.0,
                                                         resets_at:    Some(at(1_790_007_200)), },
                                           UsageWindow { used_percent: 99.0,
                                                         resets_at:    Some(at(1_789_999_000)), }], };

    let near = usage.near_limit(now).expect("the 80% window");

    assert_eq!(near.used_percent, 80.0,
               "the 99% window has reset since the snapshot; the 60% one is not near");
    assert_eq!(CodexUsage::default().near_limit(now), None);
}

/// The newest file in the newest dated directory, read from its tail - a
/// line cut by the start of the window is dropped, not misread.
#[test]
fn the_newest_session_file_is_read_from_disk() {
    let home = TempDir::new().unwrap();
    let old_day = home.path().join("sessions/2026/09/24");
    let new_day = home.path().join("sessions/2026/10/03");
    std::fs::create_dir_all(&old_day).unwrap();
    std::fs::create_dir_all(&new_day).unwrap();
    std::fs::write(old_day.join("rollout-old.jsonl"),
                   token_count(r#"{"used_percent":5.0,"resets_at":1790000000}"#, "null")).unwrap();
    // Padding far past the tail window, then the snapshot that counts.
    let padding = format!("{}\n",
                          r#"{"type":"event_msg","payload":{"type":"agent_message"}}"#);
    let mut text = padding.repeat(10_000);
    text.push_str(&token_count(r#"{"used_percent":77.0,"resets_at":1790000000}"#, "null"));
    std::fs::write(new_day.join("rollout-new.jsonl"), text).unwrap();

    let usage = read_from(home.path()).expect("read the newest file");

    assert_eq!(usage.windows[0].used_percent, 77.0);
}

#[test]
fn a_missing_or_empty_codex_home_reads_as_nothing() {
    let home = TempDir::new().unwrap();
    assert_eq!(read_from(home.path()), None);
    std::fs::create_dir_all(home.path().join("sessions/2026/10/03")).unwrap();
    assert_eq!(read_from(home.path()), None);
    assert_eq!(read_from(&home.path().join("nowhere")), None);
}
