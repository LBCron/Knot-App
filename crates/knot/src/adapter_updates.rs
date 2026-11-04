//! ACP adapter update checks in the background (#367): every adapter Knot can
//! install is checked shortly after launch and then once a day, a newly found
//! update is announced once, and found updates, installs and failures go to
//! the application log.
//!
//! Contract: `openspec/specs/adapter-updates/spec.md`.
//!
//! The check itself is `knot_terminal::check_adapter_update`, which the
//! Coding tab already ran on request. What is new is running it unasked: an
//! adapter going stale went unnoticed until something else broke (#598's
//! model list). The results live here, app-wide, so the Coding tab shows
//! what the last check found whichever way it ran, and a settings window
//! need not be open for a check to happen.
//!
//! The checks run `npm`, so they go to a small tokio runtime of this
//! module's own; a gpui task awaits them and records each result on the main
//! thread, where a desktop notification can be raised and the settings
//! window observes the change.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::{App, AsyncApp, BorrowAppContext, Global, SystemNotification};
use knot_agent_launch::InstallMethod;
use knot_terminal::AdapterUpdateStatus;

use crate::consts::{
    ADAPTER_UPDATE_CHECK_INTERVAL, ADAPTER_UPDATE_FIRST_CHECK_DELAY,
    ADAPTER_UPDATE_NOTIFICATION_TAG_PREFIX,
};

/// How a result was arrived at: unasked, or by the user checking or updating
/// from the Coding tab, who has seen it already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Found {
    Background,
    User,
}

/// What the checks have found, per agent type.
#[derive(Default)]
pub(crate) struct AdapterUpdates {
    /// Runs the checks; `None` in tests, or if it could not be started.
    runtime:   Option<Arc<tokio::runtime::Runtime>>,
    /// The latest result per agent type.
    latest:    BTreeMap<String, AdapterUpdateStatus>,
    /// The version announced per agent type, so one is announced once.
    announced: BTreeMap<String, String>,
}

impl Global for AdapterUpdates {}

impl AdapterUpdates {
    /// Records `status` for `agent_type`, and reports whether it is an update
    /// to announce: available, found unasked, and not announced at this
    /// version before. A version the user found is marked as announced too,
    /// so a later background check does not tell them again.
    pub(crate) fn note(&mut self, agent_type: &str, status: AdapterUpdateStatus, found: Found)
                       -> bool {
        let fresh = status.update_available
                    && self.announced.get(agent_type) != Some(&status.latest_version);
        if status.update_available {
            self.announced
                .insert(agent_type.to_owned(), status.latest_version.clone());
        }
        self.latest.insert(agent_type.to_owned(), status);
        fresh && found == Found::Background
    }

    /// The latest result for `agent_type`, if it has been checked.
    pub(crate) fn latest(&self, agent_type: &str) -> Option<&AdapterUpdateStatus> {
        self.latest.get(agent_type)
    }

    /// Whether `agent_type`'s adapter has an update waiting.
    pub(crate) fn update_available(&self, agent_type: &str) -> bool {
        self.latest(agent_type)
            .is_some_and(|status| status.update_available)
    }

    /// Whether any adapter has an update waiting.
    pub(crate) fn any_update_available(&self) -> bool {
        self.latest.values().any(|status| status.update_available)
    }
}

/// The adapters Knot can install, and so check: every agent type whose ACP
/// adapter declares an npm install.
pub(crate) fn checkable() -> Vec<(&'static str, InstallMethod)> {
    knot_core::agent_type::ALL.iter()
                              .filter(|kind| !kind.is_custom)
                              .filter_map(|kind| {
                                  knot_agent_launch::acp_adapter(kind.id).and_then(|config| {
                                                                             config.install
                                                                         })
                                                                         .map(|install| {
                                                                             (kind.id, install)
                                                                         })
                              })
                              .collect()
}

/// Sets up the state, so the Coding tab can read and record results. No
/// checks run; see [`install`].
pub(crate) fn init(cx: &mut App) {
    if !cx.has_global::<AdapterUpdates>() {
        cx.set_global(AdapterUpdates::default());
    }
}

/// Sets up the state and starts the background checks.
pub(crate) fn install(cx: &mut App) {
    init(cx);
    let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(1)
                                                                   .thread_name("adapter-updates")
                                                                   .enable_all()
                                                                   .build()
    {
        Ok(runtime) => Arc::new(runtime),
        Err(error) => {
            eprintln!("adapter updates: could not start the checker: {error}");
            return;
        }
    };
    cx.global_mut::<AdapterUpdates>().runtime = Some(Arc::clone(&runtime));
    cx.spawn(async move |cx| {
          cx.background_executor()
            .timer(ADAPTER_UPDATE_FIRST_CHECK_DELAY)
            .await;
          loop {
              check_all(&runtime, cx).await;
              cx.background_executor()
                .timer(ADAPTER_UPDATE_CHECK_INTERVAL)
                .await;
          }
      })
      .detach();
}

/// Checks every adapter in turn, recording each result as it arrives. One
/// failing does not stop the rest, nor the next round.
async fn check_all(runtime: &tokio::runtime::Runtime, cx: &mut AsyncApp) {
    for (agent_type, install) in checkable() {
        let outcome = runtime.spawn(knot_terminal::check_adapter_update(install))
                             .await;
        cx.update(|cx| match outcome {
              Ok(Ok(status)) => record(cx, agent_type, status, Found::Background),
              Ok(Err(error)) => log_failure(install, &error.to_string()),
              Err(error) => log_failure(install, &error.to_string()),
          });
    }
}

/// Records a check's result for `agent_type`: kept for the Coding tab,
/// logged when it finds an update, and announced when [`AdapterUpdates::note`]
/// says so.
pub(crate) fn record(cx: &mut App, agent_type: &str, status: AdapterUpdateStatus, found: Found) {
    init(cx);
    let package =
        knot_agent_launch::acp_adapter(agent_type).and_then(|config| config.install)
                                                  .map_or(agent_type, |install| install.package);
    let installed = status.installed_version
                          .clone()
                          .unwrap_or_else(|| "not installed".to_owned());
    let latest = status.latest_version.clone();
    let available = status.update_available;
    let announce =
        cx.update_global::<AdapterUpdates, _>(|updates, _| updates.note(agent_type, status, found));
    if available {
        eprintln!("adapter updates: {package} {installed} -> {latest} available");
    }
    if announce && crate::settings_global::read(cx).desktop_notifications_enabled {
        cx.show_system_notification(SystemNotification {
              tag:     format!("{ADAPTER_UPDATE_NOTIFICATION_TAG_PREFIX}{agent_type}").into(),
              title:   knot_core::l10n::t("adapter_updates.notification_title").into(),
              body:    knot_core::l10n::t_with("adapter_updates.notification_body",
                                               &[("adapter", package),
                                                 ("version", &latest)])
                                       .into(),
              actions: Vec::new(),
          });
    }
}

/// Logs an install's outcome and records the version it left.
pub(crate) fn record_install(cx: &mut App, agent_type: &str,
                             outcome: Result<AdapterUpdateStatus, String>) {
    let package =
        knot_agent_launch::acp_adapter(agent_type).and_then(|config| config.install)
                                                  .map_or(agent_type, |install| install.package);
    match outcome {
        Ok(status) => {
            let version = status.installed_version
                                .clone()
                                .unwrap_or_else(|| "an unknown version".to_owned());
            eprintln!("adapter updates: installed {package} {version}");
            record(cx, agent_type, status, Found::User);
        }
        Err(error) => eprintln!("adapter updates: installing {package} failed: {error}"),
    }
}

/// Logs a check that failed.
pub(crate) fn log_failure(install: InstallMethod, error: &str) {
    eprintln!("adapter updates: checking {} failed: {error}",
              install.package);
}

#[cfg(test)]
mod tests;
