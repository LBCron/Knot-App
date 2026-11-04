//! The Coding tab (#367; `settings-ui`, "Coding tab"): agent tabs, each
//! agent's options in its own group, and an adapter update a background
//! check found showing on the tab without a click.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::Entity;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::component::Root;
use knot_terminal::AdapterUpdateStatus;
use tempfile::TempDir;

use crate::adapter_updates::Found;
use crate::settings_window::SettingsTab;
use crate::settings_window::SettingsWindow;
use crate::settings_window::open_settings_window;

struct Fixture {
    window: VisualTestContext,
    view:   Entity<SettingsWindow>,
    _dir:   TempDir,
}

/// A settings window on its Coding tab, with Claude's and Codex's options
/// set apart.
fn coding_tab(cx: &mut TestAppContext) -> Fixture {
    let dir = TempDir::new().expect("a temporary settings root");
    let handle: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    let (window, view) = cx.update(|cx| {
                               gpui_kit::init(cx);
                               let mut settings = knot_core::Settings::with_store_root(dir.path());
                               settings.agent_options
                                       .insert("claude".into(), "--claude-flag".into());
                               settings.agent_options
                                       .insert("codex".into(), "--codex-flag".into());
                               crate::settings_global::install(settings, cx);
                               open_settings_window(&handle, cx);
                               let window = handle.borrow().expect("the settings window opened");
                               let view = window.downcast::<Root>()
                                                .expect("a Root")
                                                .read(cx)
                                                .expect("open")
                                                .view()
                                                .clone()
                                                .downcast::<SettingsWindow>()
                                                .expect("the settings view");
                               (window, view)
                           });
    let mut window = VisualTestContext::from_window(window, cx);
    view.update(&mut window, |view, cx| {
            view.selected_tab = SettingsTab::Coding;
            cx.notify();
        });
    window.run_until_parked();
    Fixture { window,
              view,
              _dir: dir }
}

impl Fixture {
    fn options_shown(&mut self) -> String {
        let view = self.view.clone();
        self.window.update(|_, cx| {
                       view.read(cx)
                           .agent_options_input
                           .read(cx)
                           .value()
                           .to_string()
                   })
    }
}

#[gpui_kit::test]
fn switching_agent_tabs_shows_that_agents_options(cx: &mut TestAppContext) {
    let mut fixture = coding_tab(cx);
    assert_eq!(fixture.options_shown(), "--claude-flag");

    fixture.view
           .update_in(&mut fixture.window, |view, window, cx| {
               view.select_agent_type("codex", window, cx)
           });
    fixture.window.run_until_parked();

    assert_eq!(fixture.options_shown(), "--codex-flag");
}

/// A background check's finding marks the tabs with no click, and only
/// while an update waits.
#[gpui_kit::test]
fn a_background_finding_marks_the_tabs(cx: &mut TestAppContext) {
    let mut fixture = coding_tab(cx);
    assert!(fixture.window.debug_bounds("adapter-update-dot").is_none());

    fixture.window.update(|_, cx| {
                      crate::adapter_updates::record(cx,
                                                     "claude",
                                                     AdapterUpdateStatus { installed_version:
                                                                               Some("1.0.0".into()),
                                                                           latest_version:
                                                                               "1.1.0".into(),
                                                                           update_available:
                                                                               true, },
                                                     Found::Background)
                  });
    fixture.window.run_until_parked();

    assert!(fixture.window.debug_bounds("adapter-update-dot").is_some(),
            "the window repaints on the finding and shows the dot");
}
