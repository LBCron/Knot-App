//! The folder each agent's git state was last read from (#25).
//!
//! The git panel's status and diffs, the header's diff stat and the
//! dashboard cards' are cached by agent, not by path: an agent has one
//! working tree at a time, so the id is the natural key. But the tree an
//! agent's git is read from can change under it - its folder edited, and
//! in time the worktree it says it is working in - and an id-keyed cache
//! then served the old tree's status until the entry aged out, while the
//! panel's watch kept following the old tree for good.
//!
//! Each reader of those caches tells this which folder it is about to read
//! for an agent, and is told when that is a different folder from last
//! time, so it can drop what it read before.

use std::collections::BTreeMap;

use uuid::Uuid;

/// The folder each agent's git state was last read from.
#[derive(Debug, Default)]
pub(crate) struct ReadFolders(BTreeMap<Uuid, String>);

impl ReadFolders {
    /// Records `folder` as the one `id`'s git is read from now, and reports
    /// whether it replaced a different one - the caller's cue to forget
    /// what it read from that. An agent seen for the first time has nothing
    /// to forget.
    pub(crate) fn moved(&mut self, id: Uuid, folder: &str) -> bool {
        match self.0.get_mut(&id) {
            Some(known) if known == folder => false,
            Some(known) => {
                folder.clone_into(known);
                true
            }
            None => {
                self.0.insert(id, folder.to_owned());
                false
            }
        }
    }

    /// Forgets `id`, as its other per-agent state is forgotten.
    pub(crate) fn forget(&mut self, id: Uuid) {
        self.0.remove(&id);
    }
}

#[cfg(test)]
mod tests;
