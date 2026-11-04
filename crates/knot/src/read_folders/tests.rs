//! Unit tests for [`super`].

use uuid::Uuid;

use super::ReadFolders;

#[test]
fn a_first_sighting_is_not_a_move() {
    let mut folders = ReadFolders::default();
    assert!(!folders.moved(Uuid::new_v4(), "/repo"));
}

#[test]
fn the_same_folder_again_is_not_a_move() {
    let (mut folders, id) = (ReadFolders::default(), Uuid::new_v4());
    folders.moved(id, "/repo");
    assert!(!folders.moved(id, "/repo"));
}

#[test]
fn a_different_folder_is_a_move_once() {
    let (mut folders, id) = (ReadFolders::default(), Uuid::new_v4());
    folders.moved(id, "/repo");
    assert!(folders.moved(id, "/repo-worktree"));
    assert!(!folders.moved(id, "/repo-worktree"),
            "reported once, then current");
}

#[test]
fn agents_are_tracked_apart() {
    let mut folders = ReadFolders::default();
    let (one, two) = (Uuid::new_v4(), Uuid::new_v4());
    folders.moved(one, "/a");
    folders.moved(two, "/b");
    assert!(!folders.moved(one, "/a"));
    assert!(folders.moved(two, "/a"));
}

#[test]
fn a_forgotten_agent_starts_over() {
    let (mut folders, id) = (ReadFolders::default(), Uuid::new_v4());
    folders.moved(id, "/a");
    folders.forget(id);
    assert!(!folders.moved(id, "/b"),
            "a fresh sighting, nothing to forget");
}
