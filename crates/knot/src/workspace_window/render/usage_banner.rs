//! The banner over a Codex agent's pane when its usage allowance is nearly
//! spent (#291): which window, when it resets, and where to get more - like
//! Codex's own "Approaching usage limit" notice.
//!
//! The figures come from Codex's session files (`crate::codex_usage`), read
//! off the main thread into `codex_usage` by `refresh_codex_usage`, outside
//! `render/`; this module only draws what landed.

use std::time::SystemTime;

use gpui_kit::AnyElement;
use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;

use crate::consts::CODEX_USAGE_URL;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// `pane` with the usage banner above it, when the selected agent is a
    /// Codex one, a window is near its limit and the user has not dismissed
    /// the banner for that reset.
    pub(in crate::workspace_window) fn with_usage_banner(&self, pane: AnyElement,
                                                         cx: &mut Context<Self>)
                                                         -> AnyElement {
        let codex_selected = self.selected_agent.is_some_and(|id| {
                                                    self.store
                                                        .lock()
                                                        .agent(id)
                                                        .is_some_and(|agent| {
                                                            agent.agent_type
                                                            == knot_core::agent_type::CODEX
                                                        })
                                                });
        let usage = self.codex_usage.get(&()).flatten();
        let near = usage.as_ref()
                        .filter(|_| codex_selected)
                        .and_then(|usage| usage.near_limit(SystemTime::now()));
        let Some(window) =
            near.filter(|window| self.codex_usage_dismissed != Some(window.resets_at))
        else {
            return pane;
        };
        let resets_at = window.resets_at;
        let resets = resets_at.map(|at| {
                                  knot_core::l10n::t_with("usage.resets_at",
                                                 &[("when",
                                                    &crate::timestamp::absolute_timestamp(at))])
                              });
        let banner =
            h_flex().id("codex-usage-banner")
                    .debug_selector(|| "codex-usage-banner".into())
                    .flex_shrink_0()
                    .w_full()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .items_center()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted)
                    .text_sm()
                    .child(Icon::new(IconName::Gauge).small()
                                                     .text_color(cx.theme().muted_foreground))
                    .child(div().child(knot_core::l10n::t("usage.approaching_limit")))
                    .children(resets.map(|resets| {
                                         div().flex_1()
                                              .min_w_0()
                                              .text_color(cx.theme().muted_foreground)
                                              .child(resets)
                                     }))
                    .child(Button::new("codex-usage-more").label(knot_core::l10n::t("usage.get_more"))
                                                           .small()
                                                           .on_click(|_: &ClickEvent, _, _| {
                                                               crate::open_in::open_url(CODEX_USAGE_URL);
                                                           }))
                    .child(Button::new("codex-usage-dismiss").icon(IconName::Close)
                                                              .tooltip(knot_core::l10n::t("usage.dismiss"))
                                                              .ghost()
                                                              .xsmall()
                                                              .on_click(cx.listener(move |view,
                                                                                           _: &ClickEvent,
                                                                                           _,
                                                                                           cx| {
                                                                  view.codex_usage_dismissed =
                                                                      Some(resets_at);
                                                                  cx.notify();
                                                              })));
        v_flex().size_full()
                .child(banner)
                .child(div().flex_1().min_h_0().child(pane))
                .into_any_element()
    }
}

#[cfg(test)]
mod tests;
