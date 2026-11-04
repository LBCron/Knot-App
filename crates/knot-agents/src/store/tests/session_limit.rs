//! `Agent::session_limit`: the usage limit that stopped an agent, kept
//! exactly as long as the agent shows it (#318).

use super::super::*;
use crate::SessionLimit;

fn limit() -> SessionLimit {
    SessionLimit { resets: Some("5:20pm".to_owned()), }
}

#[test]
fn a_limit_is_shown_while_the_agent_is_in_error() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());

    store.set_session_limit(id, Some(limit()));
    assert_eq!(store.agent(id).unwrap().active_session_limit(),
               None,
               "not shown until the agent is in Error");
    store.set_state(id, AgentState::Error);

    assert_eq!(store.agent(id).unwrap().active_session_limit(),
               Some(&limit()));
}

/// The next prompt moves the agent on, and the limit goes with it.
#[test]
fn leaving_error_clears_the_limit() {
    for next in [AgentState::Running, AgentState::Idle, AgentState::Input] {
        let mut store = AgentStore::new();
        let id = store.create("/tmp/a", CreateOptions::default());
        store.set_session_limit(id, Some(limit()));
        store.set_state(id, AgentState::Error);

        store.set_state(id, next);
        store.set_state(id, AgentState::Error);

        assert_eq!(store.agent(id).unwrap().active_session_limit(),
                   None,
                   "an Error after {next:?} is not the old limit");
    }
}

#[test]
fn a_new_limit_moves_the_revision_and_a_repeat_does_not() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    let start = store.revision();

    store.set_session_limit(id, Some(limit()));
    let after = store.revision();
    assert_ne!(after, start);

    store.set_session_limit(id, Some(limit()));
    assert_eq!(store.revision(), after);
}

/// A terminal agent's trackers report what its wrap-up does; none of it
/// lifts the limit (#36).
#[test]
fn tracked_states_do_not_lift_a_limit() {
    for next in [AgentState::Running, AgentState::Idle, AgentState::Input] {
        let mut store = AgentStore::new();
        let id = store.create("/tmp/a", CreateOptions::default());
        store.set_session_limit(id, Some(limit()));
        store.set_state(id, AgentState::Error);

        store.set_tracked_state(id, next);

        let agent = store.agent(id).unwrap();
        assert_eq!(agent.state, AgentState::Error, "{next:?} landed");
        assert_eq!(agent.active_session_limit(), Some(&limit()));
    }
}

/// Clearing the limit - a submitted prompt - lets the next report land.
#[test]
fn a_cleared_limit_lets_tracked_states_land() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_session_limit(id, Some(limit()));
    store.set_state(id, AgentState::Error);

    store.set_session_limit(id, None);
    store.set_tracked_state(id, AgentState::Running);

    assert_eq!(store.agent(id).unwrap().state, AgentState::Running);
}

/// Without a limit shown, a tracked state is any other state write.
#[test]
fn tracked_states_land_without_a_limit() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    // Recorded but not shown: a panel agent's limit lands before its Error.
    store.set_session_limit(id, Some(limit()));

    store.set_tracked_state(id, AgentState::Running);

    assert_eq!(store.agent(id).unwrap().state, AgentState::Running);
}
