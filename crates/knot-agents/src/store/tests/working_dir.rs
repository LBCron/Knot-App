//! `AgentStore::set_working_dir`: where an agent says it is working (#25),
//! read through `Agent::effective_folder`, revised so windows repaint, and
//! cleared by every restart.

use super::super::*;

fn agent() -> (AgentStore, Uuid) {
    let mut store = AgentStore::new();
    let id = store.create("/repo", CreateOptions::default());
    (store, id)
}

fn effective(store: &AgentStore, id: Uuid) -> &str {
    store.agent(id).expect("the agent").effective_folder()
}

#[test]
fn the_folder_is_read_until_a_working_directory_is_set() {
    let (mut store, id) = agent();
    assert_eq!(effective(&store, id), "/repo");

    store.set_working_dir(id, Some("/repo-feature".to_string()));
    assert_eq!(effective(&store, id), "/repo-feature");

    store.set_working_dir(id, None);
    assert_eq!(effective(&store, id), "/repo");
}

#[test]
fn its_own_folder_is_no_working_directory() {
    let (mut store, id) = agent();
    store.set_working_dir(id, Some("/repo".to_string()));
    assert_eq!(store.agent(id).expect("the agent").working_dir, None);
}

#[test]
fn a_change_moves_the_revision_and_a_repeat_does_not() {
    let (mut store, id) = agent();
    let start = store.revision();

    store.set_working_dir(id, Some("/repo-feature".to_string()));
    let after = store.revision();
    assert_ne!(after, start, "the windows reading git have to repaint");

    store.set_working_dir(id, Some("/repo-feature".to_string()));
    assert_eq!(store.revision(),
               after,
               "the same directory again costs nothing");
}

#[test]
fn every_restart_clears_it() {
    let (mut store, id) = agent();
    store.set_working_dir(id, Some("/repo-feature".to_string()));
    store.restart(id).expect("restart");
    assert_eq!(effective(&store, id), "/repo");

    store.set_working_dir(id, Some("/repo-feature".to_string()));
    store.restart_keeping_conversation(id).expect("restart");
    assert_eq!(effective(&store, id),
               "/repo",
               "a kept conversation starts in the folder too");
}
