//! The markdown section's earlier files (#29): which are listed, and that
//! choosing one shows it again.

use std::path::PathBuf;

use gpui_kit::TestAppContext;

use super::earlier_files;
use crate::workspace_window::shortcuts_tests::window_with_agents;

fn file(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/{name}.md"))
}

#[test]
fn the_current_file_is_not_an_earlier_one() {
    let history = [file("third"), file("second"), file("first")];
    assert_eq!(earlier_files(&history, &file("third")),
               [file("second"), file("first")]);
}

#[test]
fn a_file_shown_again_keeps_its_place_in_the_order() {
    let history = [file("first"), file("second")];
    assert_eq!(earlier_files(&history, &file("second")), [file("first")]);
}

#[test]
fn a_single_file_has_nothing_earlier() {
    assert!(earlier_files(&[file("only")], &file("only")).is_empty());
}

/// The agent shows one file, then another over it; choosing the first from
/// the section's list brings it back, and keeps the second within reach.
#[gpui_kit::test]
fn choosing_an_earlier_file_shows_it_again(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, cx| {
                    let mut store = view.store.lock();
                    store.set_markdown_panel(id, file("reading"), false)
                         .expect("the agent exists");
                    store.set_markdown_panel(id, file("newer"), true)
                         .expect("the agent exists");
                    drop(store);
                    cx.notify();
                });

    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.show_earlier_markdown(id, &file("reading"), cx)
                });

    let (shown, history) = fixture.view.read_with(&fixture.window, |view, _| {
                                           let store = view.store.lock();
                                           let agent = store.agent(id).expect("the agent");
                                           (agent.markdown_file.clone(),
                                            agent.markdown_history.clone())
                                       });
    assert_eq!(shown, Some(file("reading")));
    assert_eq!(earlier_files(&history, &file("reading")), [file("newer")]);
}

#[test]
fn the_control_copy_resolves() {
    let key = "artifact_panel.earlier_files";
    assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
}
