//! `set-working-directory`: an agent telling Knot which git working tree it is
//! working in (#25), so the git panel and diff stats read that tree rather than
//! the folder the agent started in.
//!
//! Contract: `openspec/specs/mcp-tools/spec.md`, "set-working-directory".
//!
//! Knot cannot see the move itself: an ACP session's cwd is fixed when it
//! opens, and an agent's `cd` does not change its own process's directory.
//! The agent has to say. The value is runtime only; a restart or deactivation
//! clears it.

use std::path::Path;

use knot_agents::AgentStore;
use knot_mcp::ToolCallResult;

use crate::args::require_str;
use crate::lookup::{agent_not_found, find_by_name_or_id};
use crate::repos::requested_destination;

pub fn set_working_directory(store: &mut AgentStore, arguments: &serde_json::Value)
                             -> ToolCallResult {
    let agent_id = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let path = match require_str(arguments, "path") {
        Ok(value) => value.trim(),
        Err(error) => return error,
    };
    let Some(agent) = find_by_name_or_id(store, agent_id)
    else {
        return agent_not_found(store, agent_id);
    };
    let (id, folder) = (agent.id, agent.folder.clone());
    if path.is_empty() {
        store.set_working_dir(id, None);
        return ToolCallResult::ok(format!("Working directory cleared; git is read from your folder, {folder}"));
    }
    let home = std::env::home_dir();
    let dir = match requested_destination(path, home.as_deref()) {
        Ok(dir) => dir,
        Err(error) => return ToolCallResult::error(error.to_string()),
    };
    if !dir.is_dir() {
        return ToolCallResult::error(format!("{} does not exist or is not a folder",
                                             dir.display()));
    }
    if !knot_git::is_working_tree(&dir) {
        return ToolCallResult::error(format!(
            "{} is not the root of a git working tree. Pass the worktree's root, the folder \
             that holds its .git.",
            dir.display()
        ));
    }
    let dir = without_trailing_slash(&dir);
    // Its own folder, however it was spelled, is going home.
    let own =
        requested_destination(&folder, home.as_deref()).ok()
                                                       .map(|own| without_trailing_slash(&own));
    if own.as_deref() == Some(dir.as_str()) {
        store.set_working_dir(id, None);
        return ToolCallResult::ok(format!("Working directory cleared; git is read from your folder, {folder}"));
    }
    store.set_working_dir(id, Some(dir.clone()));
    ToolCallResult::ok(format!("Working directory set to {dir}"))
}

/// `path` as text, without a trailing `/` (but `/` itself kept), so two
/// spellings of one folder compare equal.
fn without_trailing_slash(path: &Path) -> String {
    let text = path.to_string_lossy();
    match text.trim_end_matches('/') {
        "" => "/".to_owned(),
        trimmed => trimmed.to_owned(),
    }
}

#[cfg(test)]
mod tests;
