//! The markdown section's way back to the files the agent showed before the
//! current one (#29): an agent showing a new file replaces the one being
//! read, and the user should not have to find the agent's context menu to
//! get it back.
//!
//! Contract: `openspec/specs/artifact-panel/spec.md`, "The markdown section
//! lists the agent's earlier files".
//!
//! The list is the agent's markdown history - what the Markdown Files menu
//! reads - so the two cannot disagree, and choosing a file shows it the way
//! that menu does, through the store.

use std::path::{Path, PathBuf};

use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// The files in `history` (most recent first) other than `current`, in the
/// same order.
pub(super) fn earlier_files(history: &[PathBuf], current: &Path) -> Vec<PathBuf> {
    history.iter()
           .filter(|file| file.as_path() != current)
           .cloned()
           .collect()
}

impl WorkspaceWindow {
    /// The section's earlier-files control: a menu of the files the agent
    /// showed before `current`, most recent first, or none when there are
    /// none.
    pub(super) fn markdown_history_action(&self, id: Uuid, current: &Path,
                                          cx: &mut Context<Self>)
                                          -> Option<gpui_kit::AnyElement> {
        let history = self.store
                          .lock()
                          .agent(id)
                          .map(|agent| agent.markdown_history.clone())
                          .unwrap_or_default();
        let earlier = earlier_files(&history, current);
        if earlier.is_empty() {
            return None;
        }
        let entity = cx.entity();
        Some(Button::new(super::controls::element("markdown-history", id))
                 .icon(IconName::Files)
                 .ghost()
                 .small()
                 .tooltip(knot_core::l10n::t("artifact_panel.earlier_files"))
                 .dropdown_menu(move |mut menu, _, _| {
                     for file in &earlier {
                         let (entity, file) = (entity.clone(), file.clone());
                         menu = menu.item(PopupMenuItem::new(crate::agent_menu::markdown_label(&file))
                                              .on_click(move |_, _, app| {
                                                  entity.update(app, |view, cx| {
                                                      view.show_earlier_markdown(id, &file, cx)
                                                  });
                                              }));
                     }
                     menu
                 })
                 .into_any_element())
    }

    /// Shows `file` in `id`'s markdown section again, unexpanded - the user
    /// asks for nothing about its size - as the Markdown Files menu does.
    pub(in crate::workspace_window) fn show_earlier_markdown(&mut self, id: Uuid, file: &Path,
                                                             cx: &mut Context<Self>) {
        if let Err(error) = self.store
                                .lock()
                                .set_markdown_panel(id, file.to_path_buf(), false)
        {
            eprintln!("failed to show {}: {error}", file.display());
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
