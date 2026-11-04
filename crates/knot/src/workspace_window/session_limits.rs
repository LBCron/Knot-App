//! A terminal agent's usage limit, from its output to its status (#36).
//!
//! `TerminalSession` reads the notice on the PTY reader thread, which knows
//! neither the agent id nor the store; this is where the two meet. The
//! store then holds the agent in Error until a submitted prompt clears the
//! limit - see `AgentStore::set_tracked_state`. A Panel-mode agent reaches
//! the same state from its prompt's error instead, in
//! `sync_panel_agent_states`.

use uuid::Uuid;

use super::WorkspaceWindow;

impl WorkspaceWindow {
    /// Take every terminal session's announced limit into the store,
    /// returning whether any landed; and re-arm the scan of every session
    /// whose limit has since been lifted.
    ///
    /// The limit is written before the state, as the panel path does, so
    /// the frame that turns the dot red already says why. The re-arm is what
    /// lets a prompt refused again after a lift - with the same "resets" as
    /// the refusal before it - show the limit again, rather than read as the
    /// old notice redrawn.
    pub(super) fn drain_session_limits(&mut self) -> bool {
        let seen = self.sessions
                       .iter()
                       .filter_map(|(id, session)| {
                           session.lock()
                                  .take_session_limit()
                                  .map(|limit| (*id, limit))
                       })
                       .collect::<Vec<(Uuid, _)>>();
        let landed = !seen.is_empty();
        let lifted = {
            let mut store = self.store.lock();
            for (id, limit) in seen {
                store.set_session_limit(id, Some(limit));
                store.set_state(id, knot_agents::AgentState::Error);
            }
            self.sessions
                .keys()
                .filter(|id| {
                    store.agent(**id)
                         .is_some_and(|agent| agent.session_limit.is_none())
                })
                .copied()
                .collect::<Vec<Uuid>>()
        };
        for id in lifted {
            if let Some(session) = self.sessions.get(&id) {
                session.lock().rearm_session_limit();
            }
        }
        landed
    }
}
