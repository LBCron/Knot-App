//! A terminal agent's hooks during and after a usage limit (#36): the
//! wrap-up's hooks keep the limit, a submitted prompt lifts it.

use std::sync::Arc;

use knot_agents::{AgentState, AgentStore, SessionLimit};
use knot_mcp::{AgentHookHandler, HookRequest};
use knot_messaging::NoopNotifier;
use parking_lot::Mutex;
use tokio::sync::watch;
use uuid::Uuid;

use crate::McpToolCatalog;

fn catalog() -> McpToolCatalog {
    let (_tx, rx) = watch::channel(Vec::new());
    McpToolCatalog::new(Arc::new(Mutex::new(AgentStore::new())),
                        rx,
                        Arc::new(NoopNotifier))
}

fn limited(cat: &McpToolCatalog) -> Uuid {
    let mut agents = cat.agents.lock();
    let id = agents.create("/tmp/a", knot_agents::CreateOptions::default());
    agents.set_session_limit(id, Some(SessionLimit { resets: None }));
    agents.set_state(id, AgentState::Error);
    id
}

fn hook(id: Uuid, hook: &str, status: &str) -> HookRequest {
    serde_json::from_value(serde_json::json!({ "agent_id": id,
                                               "agent": "claude",
                                               "hook": hook,
                                               "status": status })).unwrap()
}

#[tokio::test]
async fn the_wrap_up_hooks_keep_the_limit() {
    let cat = catalog();
    let id = limited(&cat);

    for (name, status) in [("PreToolUse", "running"), ("Stop", "idle")] {
        cat.status(&hook(id, name, status)).unwrap();
        tokio::task::yield_now().await;

        let agents = cat.agents.lock();
        let agent = agents.agent(id).unwrap();
        assert_eq!(agent.state, AgentState::Error, "{name} moved the agent");
        assert!(agent.active_session_limit().is_some(),
                "{name} lifted the limit");
    }
}

#[tokio::test]
async fn a_submitted_prompt_lifts_the_limit() {
    let cat = catalog();
    let id = limited(&cat);

    cat.status(&hook(id, "UserPromptSubmit", "running"))
       .unwrap();
    tokio::task::yield_now().await;

    let agents = cat.agents.lock();
    let agent = agents.agent(id).unwrap();
    assert_eq!(agent.state, AgentState::Running);
    assert_eq!(agent.session_limit, None);
}

/// A permission the wrap-up asks for shows over the limit, which is back
/// once the wrap-up carries on (#377 review).
#[tokio::test]
async fn a_permission_prompt_shows_over_the_limit() {
    let cat = catalog();
    let id = limited(&cat);

    cat.status(&hook(id, "PermissionRequest", "input")).unwrap();
    tokio::task::yield_now().await;
    assert_eq!(cat.agents.lock().agent(id).unwrap().state,
               AgentState::Input);

    cat.status(&hook(id, "PreToolUse", "running")).unwrap();
    tokio::task::yield_now().await;
    let agents = cat.agents.lock();
    let agent = agents.agent(id).unwrap();
    assert_eq!(agent.state, AgentState::Error);
    assert!(agent.active_session_limit().is_some());
}
