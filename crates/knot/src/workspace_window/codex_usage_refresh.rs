//! Keeping Codex's usage figures fresh while a Codex agent is in the window
//! (#291). Outside `render/` because it starts work: the frame calls it, and
//! it hands the read straight to `spawn_blocking`.

use crate::consts::CODEX_USAGE_MAX_AGE;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Re-read Codex's usage once it has aged out, while any agent in the
    /// window is a Codex one. The read is a directory listing and a file
    /// tail, so the writer goes straight to `spawn_blocking`.
    pub(in crate::workspace_window) fn refresh_codex_usage(&mut self) {
        // The ids first: `workspace_agent_ids` takes the store lock itself.
        let ids = self.workspace_agent_ids();
        let has_codex = {
            let store = self.store.lock();
            ids.iter().any(|id| {
                          store.agent(*id).is_some_and(|agent| {
                                              agent.agent_type == knot_core::agent_type::CODEX
                                          })
                      })
        };
        if !has_codex {
            return;
        }
        if let Some(writer) = self.codex_usage.claim_refresh((), CODEX_USAGE_MAX_AGE) {
            let read = self.codex_usage_source;
            self.runtime.spawn_blocking(move || writer.record(read()));
        }
    }
}
