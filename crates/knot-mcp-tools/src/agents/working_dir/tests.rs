//! Unit tests for [`super`]: what `set-working-directory` accepts, refuses,
//! and clears.

use knot_agents::{AgentStore, CreateOptions};
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

use super::set_working_directory;

/// A folder holding a `.git`, which is all `is_working_tree` asks of a root.
fn working_tree() -> TempDir {
    let dir = TempDir::new().expect("a temporary folder");
    std::fs::create_dir(dir.path().join(".git")).expect("a .git folder");
    dir
}

fn agent_in(folder: &str) -> (AgentStore, Uuid) {
    let mut store = AgentStore::new();
    let id = store.create(folder, CreateOptions::default());
    (store, id)
}

fn call(store: &mut AgentStore, id: Uuid, path: &str) -> knot_mcp::ToolCallResult {
    set_working_directory(store, &json!({"agentId": id.to_string(), "path": path}))
}

fn effective(store: &AgentStore, id: Uuid) -> String {
    store.agent(id)
         .expect("the agent")
         .effective_folder()
         .to_owned()
}

#[test]
fn a_working_tree_root_becomes_the_working_directory() {
    let (home, worktree) = (working_tree(), working_tree());
    let (mut store, id) = agent_in(&home.path().to_string_lossy());
    let path = worktree.path().to_string_lossy().into_owned();

    let result = call(&mut store, id, &format!("{path}/"));

    assert_eq!(result.is_error, None, "{}", result.content[0].text);
    assert_eq!(effective(&store, id),
               path,
               "a trailing slash is the same folder");
}

#[test]
fn a_subfolder_of_a_working_tree_is_refused() {
    let (home, worktree) = (working_tree(), working_tree());
    let inside = worktree.path().join("src");
    std::fs::create_dir(&inside).expect("a subfolder");
    let (mut store, id) = agent_in(&home.path().to_string_lossy());

    let result = call(&mut store, id, &inside.to_string_lossy());

    assert_eq!(result.is_error, Some(true));
    assert_eq!(effective(&store, id),
               home.path().to_string_lossy(),
               "unchanged");
}

#[test]
fn a_missing_or_relative_path_is_refused() {
    let home = working_tree();
    let (mut store, id) = agent_in(&home.path().to_string_lossy());

    assert_eq!(call(&mut store, id, "/no/such/folder/anywhere").is_error,
               Some(true));
    assert_eq!(call(&mut store, id, "relative/path").is_error, Some(true));
    assert_eq!(store.agent(id).expect("the agent").working_dir, None);
}

#[test]
fn an_empty_path_or_its_own_folder_goes_home() {
    let (home, worktree) = (working_tree(), working_tree());
    let folder = home.path().to_string_lossy().into_owned();
    let (mut store, id) = agent_in(&folder);
    let worktree = worktree.path().to_string_lossy().into_owned();

    call(&mut store, id, &worktree);
    assert_eq!(call(&mut store, id, "").is_error, None);
    assert_eq!(store.agent(id).expect("the agent").working_dir, None);

    call(&mut store, id, &worktree);
    assert_eq!(call(&mut store, id, &folder).is_error, None);
    assert_eq!(store.agent(id).expect("the agent").working_dir, None);
    assert_eq!(effective(&store, id), folder);
}

#[test]
fn an_unknown_agent_is_an_error() {
    let worktree = working_tree();
    let result = set_working_directory(&mut AgentStore::new(),
                                       &json!({"agentId": "nobody",
                                               "path": worktree.path().to_string_lossy()}));
    assert_eq!(result.is_error, Some(true));
}

#[test]
fn a_missing_path_is_an_error() {
    let (mut store, id) = agent_in("/tmp/a");
    let result = set_working_directory(&mut store, &json!({"agentId": id.to_string()}));
    assert_eq!(result.is_error, Some(true));
}
