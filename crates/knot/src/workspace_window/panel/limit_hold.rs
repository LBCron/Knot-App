//! Holding queued prompts while a session's usage limit stands (#368): what
//! the queued rows ask to decide whether to offer "send now", and the
//! override itself. The hold is applied where prompts are delivered - see
//! `drain_panel_prompt` and `deliver_panel_prompt` in `prompt`.

use uuid::Uuid;

use crate::panel_session;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue;

impl WorkspaceWindow {
    /// Whether `id`'s session is holding its queued prompts until its usage
    /// limit lifts (#368) - what decides whether a queued row offers to send
    /// now. A lock and a compare, no I/O, so it is safe to ask while drawing.
    pub(in crate::workspace_window) fn held_by_session_limit(&self, id: Uuid) -> bool {
        self.panel_sessions.get(&id).is_some_and(|slot| {
                                        match &*slot.lock() {
            panel_session::PanelSessionSlot::Ready(handle) => {
                handle.state()
                      .lock()
                      .held_by_session_limit(std::time::SystemTime::now())
            }
            _ => false,
        }
                                    })
    }

    /// The user's "send now" for a queued prompt the usage limit is holding
    /// (#368): it goes to the head, past the limit, on the next tick.
    pub(in crate::workspace_window) fn send_queued_prompt_now(&mut self, id: Uuid,
                                                              prompt_id: Uuid)
                                                              -> bool {
        self.panel_prompt_queues
            .get_mut(&id)
            .is_some_and(|queue| prompt_queue::send_now(queue, prompt_id))
    }
}

#[cfg(test)]
mod tests;
