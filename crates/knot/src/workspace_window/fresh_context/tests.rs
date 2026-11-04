//! When a fresh-context restart may run (#331).

use knot_agents::{AgentState, AgentStore, CreateOptions};

use super::ready_for_fresh_context;

#[test]
fn only_a_requested_agent_with_no_turn_under_way_is_restarted() {
    for (state, requested, expected) in [(AgentState::Idle, true, true),
                                         (AgentState::Error, true, true),
                                         (AgentState::Running, true, false),
                                         (AgentState::Input, true, false),
                                         (AgentState::Idle, false, false)]
    {
        let mut store = AgentStore::new();
        let id = store.create("/tmp/a", CreateOptions::default());
        store.set_state(id, state);
        if requested {
            store.request_fresh_context(id);
        }

        assert_eq!(ready_for_fresh_context(store.agent(id).unwrap()),
                   expected,
                   "{state:?}, requested: {requested}");
    }
}

/// The restart is what clears the request, so it runs once.
#[test]
fn a_fresh_restart_clears_the_request() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.request_fresh_context(id);

    store.restart(id).unwrap();

    assert!(!ready_for_fresh_context(store.agent(id).unwrap()));
}
