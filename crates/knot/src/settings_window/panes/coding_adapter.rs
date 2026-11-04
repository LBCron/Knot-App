//! The Coding tab's adapter rows: the selected agent's ACP adapter command,
//! its version status, and the check and Update actions (#598, #367).
//!
//! The rows sit in the agent's own group, beside its options (see
//! `coding`). The status is this window's own in-flight check or install
//! when there is one, and otherwise the latest result `adapter_updates`
//! holds - so a background check's finding shows here without a click.
//! Every result reached here is recorded there too, which logs it and keeps
//! the background check from announcing an update the user already saw.

use gpui_kit::AnyElement;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Disableable;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::div;

use crate::adapter_updates::{AdapterUpdates, Found};
use crate::settings_window::SettingsWindow;

/// The dot a tab carries while an adapter update is waiting (#367): on the
/// settings window's Coding tab, and on the agent's own tab inside it.
pub(in crate::settings_window) fn update_dot(cx: &gpui_kit::App) -> gpui_kit::Div {
    div().debug_selector(|| "adapter-update-dot".into())
         .ml_1()
         .size(gpui_kit::px(6.))
         .rounded_full()
         .bg(cx.theme().primary)
}

/// The Coding tab's own adapter check or install for one agent type - see
/// [`SettingsWindow::check_for_adapter_update`]/
/// [`SettingsWindow::update_adapter_now`].
#[derive(Debug, Clone, Default)]
pub(crate) enum AdapterUpdateState {
    /// Nothing running or failed in this window; the latest result, if any,
    /// is `adapter_updates`'.
    #[default]
    Idle,
    Checking,
    Checked(knot_terminal::AdapterUpdateStatus),
    Updating,
    /// Holds the check or install failure's message, shown as the row's
    /// hint instead of a version.
    Error(String),
}

impl AdapterUpdateState {
    /// The state a check or install leaves behind, given how its task on
    /// the settings window's runtime ended. The task itself failing - a
    /// panic, or the window closing mid-run and cancelling it - reads as
    /// an error like the check failing does, rather than leaving the row
    /// stuck on "Checking…".
    fn from_outcome(outcome: Result<knot_acp::Result<knot_terminal::AdapterUpdateStatus>,
                           tokio::task::JoinError>)
                    -> Self {
        match outcome {
            Ok(Ok(status)) => Self::Checked(status),
            Ok(Err(error)) => Self::Error(error.to_string()),
            Err(error) => Self::Error(error.to_string()),
        }
    }
}

impl SettingsWindow {
    /// Runs [`knot_terminal::check_adapter_update`] for `agent_type`'s
    /// npm-installed adapter on request, and records the result app-wide.
    pub(super) fn check_for_adapter_update(&mut self, agent_type: String, cx: &mut Context<Self>) {
        let Some(install) =
            knot_agent_launch::acp_adapter(&agent_type).and_then(|config| config.install)
        else {
            return;
        };
        self.adapter_update
            .insert(agent_type.clone(), AdapterUpdateState::Checking);
        cx.notify();
        let settings_window = cx.entity();
        let check = self.runtime
                        .spawn(knot_terminal::check_adapter_update(install));
        cx.spawn(async move |_this, cx| {
              let state = AdapterUpdateState::from_outcome(check.await);
              cx.update(|app| {
                    match &state {
                        AdapterUpdateState::Checked(status) => {
                            crate::adapter_updates::record(app,
                                                           &agent_type,
                                                           status.clone(),
                                                           Found::User);
                        }
                        AdapterUpdateState::Error(error) => {
                            crate::adapter_updates::log_failure(install, error);
                        }
                        _ => {}
                    }
                    settings_window.update(app, |view, cx| {
                                       view.adapter_update.insert(agent_type, state);
                                       cx.notify();
                                   });
                });
          })
          .detach();
    }

    /// Re-installs `agent_type`'s adapter (the same command the auto-install
    /// path runs on a missing binary) and re-checks its version once done,
    /// so the row reflects the new installed version without a second
    /// click. The install is logged either way.
    pub(super) fn update_adapter_now(&mut self, agent_type: String, cx: &mut Context<Self>) {
        let Some(install) =
            knot_agent_launch::acp_adapter(&agent_type).and_then(|config| config.install)
        else {
            return;
        };
        self.adapter_update
            .insert(agent_type.clone(), AdapterUpdateState::Updating);
        cx.notify();
        let settings_window = cx.entity();
        let update = self.runtime.spawn(async move {
                                     knot_terminal::update_adapter(install).await?;
                                     knot_terminal::check_adapter_update(install).await
                                 });
        cx.spawn(async move |_this, cx| {
              let state = AdapterUpdateState::from_outcome(update.await);
              cx.update(|app| {
                    let outcome = match &state {
                        AdapterUpdateState::Checked(status) => Ok(status.clone()),
                        AdapterUpdateState::Error(error) => Err(error.clone()),
                        _ => Err("the install ended without a result".to_owned()),
                    };
                    crate::adapter_updates::record_install(app, &agent_type, outcome);
                    settings_window.update(app, |view, cx| {
                                       view.adapter_update.insert(agent_type, state);
                                       cx.notify();
                                   });
                });
          })
          .detach();
    }

    /// The selected agent's adapter rows - its command and its version
    /// status with the check and Update actions - or none for a type with
    /// no declared npm install (native-ACP agents, and the user's own custom
    /// commands have nothing to check).
    pub(super) fn adapter_rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(install) =
            knot_agent_launch::acp_adapter(&self.selected_agent_type).and_then(|config| {
                                                                         config.install
                                                                     })
        else {
            return Vec::new();
        };
        let agent_type = self.selected_agent_type.clone();
        let settings_window = cx.entity();
        let mut state = self.adapter_update
                            .get(&agent_type)
                            .cloned()
                            .unwrap_or_default();
        if matches!(state, AdapterUpdateState::Idle)
           && let Some(latest) = cx.try_global::<AdapterUpdates>()
                                   .and_then(|updates| updates.latest(&agent_type))
        {
            state = AdapterUpdateState::Checked(latest.clone());
        }

        let (status_text, status_muted, can_update) = match &state {
            AdapterUpdateState::Idle => ("Not checked yet".to_string(), true, false),
            AdapterUpdateState::Checking => ("Checking…".to_string(), true, false),
            AdapterUpdateState::Updating => ("Installing update…".to_string(), true, false),
            AdapterUpdateState::Error(message) => (message.clone(), false, false),
            AdapterUpdateState::Checked(status) => {
                let installed = status.installed_version
                                      .as_deref()
                                      .unwrap_or("not installed");
                if status.update_available {
                    (format!("{installed} -> {} available", status.latest_version), false, true)
                }
                else {
                    (format!("{installed} (up to date)"), true, false)
                }
            }
        };
        let busy = matches!(state,
                            AdapterUpdateState::Checking | AdapterUpdateState::Updating);

        vec![
            Self::text_row(
                "Adapter",
                Self::mono_text(cx, install.command).text_color(cx.theme().muted_foreground),
            )
            .into_any_element(),
            Self::text_row(
                "Version",
                h_flex()
                    .gap_2()
                    .items_center()
                    .child({
                        let mut text = div().text_sm().child(status_text);
                        if status_muted {
                            text = text.text_color(cx.theme().muted_foreground);
                        } else {
                            text = text.text_color(cx.theme().danger);
                        }
                        text
                    })
                    .child(
                        crate::controls::icon_button(
                            "coding-check-adapter-update",
                            "icons/refresh-cw.svg",
                            "Check for updates",
                            false,
                        )
                        .disabled(busy)
                        .on_click({
                            let settings_window = settings_window.clone();
                            let agent_type = agent_type.clone();
                            move |_, _, app| {
                                settings_window.update(app, |view, cx| {
                                    view.check_for_adapter_update(agent_type.clone(), cx);
                                });
                            }
                        }),
                    )
                    .children(can_update.then(|| {
                        Button::new("coding-update-adapter")
                            .label("Update")
                            .small()
                            .disabled(busy)
                            .on_click({
                                let settings_window = settings_window.clone();
                                let agent_type = agent_type.clone();
                                move |_, _, app| {
                                    settings_window.update(app, |view, cx| {
                                        view.update_adapter_now(agent_type.clone(), cx);
                                    });
                                }
                            })
                    })),
            )
            .into_any_element(),
        ]
    }
}
