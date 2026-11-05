//! Every path that lets go of a panel session retires its slot, and a start
//! while one is connecting attaches to it (#382).
//!
//! The adapter-side half - a retired connect kills what it started - is
//! pinned in `panel_session::retire_tests` against real processes. These
//! pin the window's half: that restart, deactivate and a plan step's fresh
//! context (#384) all retire the slot they replace, rather than dropping it
//! and leaving its connect running. The fixture's agent type launches
//! nothing, so the slot is planted `Connecting` and nothing ever spawns.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use parking_lot::Mutex;

use super::shortcuts_tests::window_with_agents;
use crate::panel_session::{PanelPhase, PanelSessionSlot};

type Slot = Arc<Mutex<PanelSessionSlot>>;

/// Plants a `Connecting` slot for `agent`, activated, and hands it back so
/// the test can watch it after the window lets go.
fn plant_connecting(view: &mut super::WorkspaceWindow, agent: uuid::Uuid) -> Slot {
    view.store.lock().set_activated(agent, true);
    let (connecting, _progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    view.panel_sessions.insert(agent, Arc::clone(&slot));
    slot
}

fn assert_retired(slot: &Slot, path: &str) {
    assert_eq!(slot.lock().phase(),
               PanelPhase::Retired,
               "{path} retires the connecting slot, so its connect cannot finish behind the window");
}

#[gpui_kit::test]
fn restarting_a_connecting_agent_retires_its_slot(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, cx| {
                    let slot = plant_connecting(view, agent);
                    view.restart_agents(&[agent], true, cx);
                    assert_retired(&slot, "a restart");
                    assert!(view.panel_sessions
                                .get(&agent)
                                .is_none_or(|current| !Arc::ptr_eq(current, &slot)),
                            "the window no longer holds the retired slot");
                });
}

#[gpui_kit::test]
fn deactivating_a_connecting_agent_retires_its_slot(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, _| {
                    let slot = plant_connecting(view, agent);
                    view.deactivate_agent(agent);
                    assert_retired(&slot, "deactivating");
                    assert!(!view.panel_sessions.contains_key(&agent));
                });
}

/// #384's double launch: a messaged agent is started on its old
/// conversation, then the plan step's fresh context restarts it while that
/// start is still connecting. The first start must be retired, not left to
/// finish beside the second.
#[gpui_kit::test]
fn a_fresh_context_restart_retires_the_start_it_replaces(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, cx| {
                    let slot = plant_connecting(view, agent);
                    view.store.lock().request_fresh_context(agent);
                    assert!(view.restart_for_fresh_context(cx),
                            "a connecting agent is not Running, so the restart goes ahead");
                    assert_retired(&slot, "a fresh-context restart");
                });
}

/// A start requested while one is connecting - a message wake-up, a
/// reveal, the activation sweep - attaches to it rather than spawning a
/// second adapter.
#[gpui_kit::test]
fn a_start_while_connecting_attaches_to_the_slot(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, cx| {
                    let slot = plant_connecting(view, agent);
                    view.start_sessions(agent, cx);
                    view.activate_agent(agent, cx);
                    view.reveal_agent(agent, cx);
                    assert!(Arc::ptr_eq(&view.panel_sessions[&agent], &slot),
                            "every start attached to the slot already connecting");
                    assert!(matches!(slot.lock().phase(), PanelPhase::Connecting(_)),
                            "and none of them disturbed it");
                });
}
