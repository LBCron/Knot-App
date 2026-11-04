//! Unit tests for [`super`]: when a found update is announced, which
//! adapters are checked, and the notification it raises.

use gpui_kit::TestAppContext;
use knot_terminal::AdapterUpdateStatus;
use tempfile::TempDir;

use super::{AdapterUpdates, Found, checkable, record};

fn status(installed: &str, latest: &str) -> AdapterUpdateStatus {
    AdapterUpdateStatus { installed_version: Some(installed.to_owned()),
                          latest_version:    latest.to_owned(),
                          update_available:  installed != latest, }
}

#[test]
fn an_update_is_announced_once_per_version() {
    let mut updates = AdapterUpdates::default();
    assert!(updates.note("claude", status("1.0.0", "1.1.0"), Found::Background));
    assert!(!updates.note("claude", status("1.0.0", "1.1.0"), Found::Background),
            "the same version again is not news");
    assert!(updates.note("claude", status("1.0.0", "1.2.0"), Found::Background),
            "a newer version is");
}

#[test]
fn an_update_the_user_found_is_not_announced_later() {
    let mut updates = AdapterUpdates::default();
    assert!(!updates.note("codex", status("2.0.0", "2.1.0"), Found::User));
    assert!(!updates.note("codex", status("2.0.0", "2.1.0"), Found::Background),
            "they saw it on the Coding tab already");
}

#[test]
fn an_up_to_date_adapter_announces_nothing() {
    let mut updates = AdapterUpdates::default();
    assert!(!updates.note("claude", status("1.0.0", "1.0.0"), Found::Background));
    assert!(!updates.any_update_available());
}

#[test]
fn the_latest_result_says_whether_an_update_waits() {
    let mut updates = AdapterUpdates::default();
    updates.note("claude", status("1.0.0", "1.1.0"), Found::Background);
    assert!(updates.update_available("claude"));
    assert!(!updates.update_available("codex"));
    assert!(updates.any_update_available());

    updates.note("claude", status("1.1.0", "1.1.0"), Found::User);
    assert!(!updates.any_update_available(), "installed, nothing waits");
}

/// Every adapter Knot can install is checked, and nothing else: an agent
/// type with no npm install has nothing to compare.
#[test]
fn every_installable_adapter_is_checked() {
    let checked: Vec<&str> = checkable().into_iter().map(|(id, _)| id).collect();
    for kind in knot_core::agent_type::ALL.iter()
                                          .filter(|kind| !kind.is_custom)
    {
        let installable = knot_agent_launch::acp_adapter(kind.id).and_then(|config| config.install)
                                                                 .is_some();
        assert_eq!(checked.contains(&kind.id), installable, "{}", kind.id);
    }
    assert!(!checked.is_empty());
}

fn app(cx: &mut TestAppContext, notifications: bool) -> TempDir {
    let dir = TempDir::new().expect("a temporary settings root");
    let mut settings = knot_core::Settings::with_store_root(dir.path());
    settings.desktop_notifications_enabled = notifications;
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::settings_global::install(settings, cx);
          // The test platform records a notification only for an app that
          // has an identity, as the real one has.
          cx.set_app_identity("com.example.knot-tests", "Knot");
          super::init(cx);
      });
    dir
}

#[gpui_kit::test]
fn a_background_finding_raises_one_notification(cx: &mut TestAppContext) {
    let _dir = app(cx, true);
    cx.update(|cx| {
          record(cx, "claude", status("1.0.0", "1.1.0"), Found::Background);
          record(cx, "claude", status("1.0.0", "1.1.0"), Found::Background);
      });
    assert_eq!(cx.shown_system_notifications().len(), 1);
}

#[gpui_kit::test]
fn with_notifications_off_none_is_raised_but_the_update_is_kept(cx: &mut TestAppContext) {
    let _dir = app(cx, false);
    cx.update(|cx| record(cx, "claude", status("1.0.0", "1.1.0"), Found::Background));
    assert!(cx.shown_system_notifications().is_empty());
    assert!(cx.update(|cx| cx.global::<AdapterUpdates>().any_update_available()));
}

#[gpui_kit::test]
fn a_finding_on_request_raises_none(cx: &mut TestAppContext) {
    let _dir = app(cx, true);
    cx.update(|cx| record(cx, "codex", status("2.0.0", "2.1.0"), Found::User));
    assert!(cx.shown_system_notifications().is_empty());
}

#[test]
fn the_notification_copy_resolves() {
    for key in ["adapter_updates.notification_title",
                "adapter_updates.notification_body"]
    {
        assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
    }
    let body = knot_core::l10n::t_with("adapter_updates.notification_body",
                                       &[("adapter", "claude-agent-acp"), ("version", "9.9.9")]);
    assert!(body.contains("claude-agent-acp") && body.contains("9.9.9"),
            "{body}");
}
