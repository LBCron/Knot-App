//! Submitting a persona or prompt to Knot-Library (#43): a dialog asks for
//! what the library needs that Knot does not keep - a one-line description
//! and optional tags - and the item then opens on GitHub as a new file in
//! the library.
//!
//! Contract: `openspec/specs/library-windows/spec.md`, "A persona or prompt
//! can be submitted to Knot-Library".
//!
//! GitHub's new-file page does the rest: for someone who cannot write to the
//! library it forks it, and "Propose changes" opens the pull request. So
//! nothing here needs the GitHub CLI or a token, and the user reviews the
//! file in GitHub's own editor before anything is sent. The file and its
//! rules are `knot_library::Contribution`'s.

use std::rc::Rc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::ClickEvent;
use gpui_kit::ClipboardItem;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::dialog::Cancel;
use gpui_kit::component::dialog::Confirm;
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::notification::Notification;
use gpui_kit::div;
use gpui_kit::px;
use knot_core::consts::{KNOT_LIBRARY_BRANCH, KNOT_LIBRARY_REPO};
use knot_core::l10n::{t, t_with};
use knot_library::contribution::{DESCRIPTION_MAX, TAGS_MAX, TITLE_MAX};
use knot_library::{Contribution, ContributionKind, Draft, LibraryError};
use uuid::Uuid;

use crate::consts::LIBRARY_SUBMIT_URL_MAX;
use crate::open_in::percent_encode;

/// The persona or prompt a row offers for submission.
#[derive(Debug, Clone)]
pub(crate) struct Submission {
    pub(crate) kind: ContributionKind,
    pub(crate) id:   Uuid,
    pub(crate) name: String,
    /// A prompt's text, or a persona's instructions.
    pub(crate) body: String,
}

/// How handing the file to GitHub went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    /// The new-file page opened with the file in it.
    Opened,
    /// The file was too long for an address, so it went to the clipboard
    /// and the page opened empty.
    OpenedCopied,
    /// No browser opened; the file is on the clipboard.
    OpenFailed,
}

impl Outcome {
    fn notification(self) -> Notification {
        match self {
            Self::Opened => Notification::info(t("library.submit.opened")),
            Self::OpenedCopied => Notification::info(t("library.submit.opened_copied")),
            Self::OpenFailed => Notification::error(t("library.submit.open_failed")),
        }
    }
}

/// Knot-Library's new-file page for `item`, at the item's path, carrying
/// the file itself when `with_file`.
pub(super) fn new_file_url(item: &Contribution, with_file: bool) -> String {
    let mut url = format!("https://github.com/{KNOT_LIBRARY_REPO}/new/{KNOT_LIBRARY_BRANCH}/{}\
                           ?filename={}",
                          item.kind().directory(),
                          percent_encode(&item.file_name()));
    if with_file {
        url.push_str("&value=");
        url.push_str(&percent_encode(&item.markdown()));
    }
    url
}

/// Opens `item` on GitHub through `open_url`, falling back to `copy` for a
/// file too long to pass in the address, or when no browser opens.
pub(super) fn submit(item: &Contribution, open_url: impl Fn(&str) -> bool,
                     copy: impl FnOnce(String))
                     -> Outcome {
    let url = new_file_url(item, true);
    if url.len() <= LIBRARY_SUBMIT_URL_MAX {
        if open_url(&url) {
            return Outcome::Opened;
        }
        copy(item.markdown());
        return Outcome::OpenFailed;
    }
    copy(item.markdown());
    if open_url(&new_file_url(item, false)) {
        Outcome::OpenedCopied
    }
    else {
        Outcome::OpenFailed
    }
}

/// What to tell the user about a draft the library would refuse, or `None`
/// while there is nothing to say yet - before a description is typed, the
/// disabled button says enough.
pub(super) fn problem(error: &LibraryError) -> Option<String> {
    match error {
        LibraryError::InvalidTitle(_) => Some(t_with("library.submit.invalid_title",
                                                     &[("max", &TITLE_MAX.to_string())])),
        LibraryError::InvalidDescription(_) => None,
        LibraryError::InvalidTag(tag) => {
            Some(t_with("library.submit.invalid_tag", &[("tag", tag)]))
        }
        LibraryError::TooManyTags(_) => Some(t_with("library.submit.too_many_tags",
                                                    &[("max", &TAGS_MAX.to_string())])),
        _ => None,
    }
}

/// The dialog's inputs, alongside the item they describe.
struct Form {
    submission:  Submission,
    title:       Entity<InputState>,
    description: Entity<InputState>,
    tags:        Entity<InputState>,
    /// Redraw the window on each edit, so the button's enabled state and
    /// the problem line follow the typing. Dropped with the dialog.
    _edits:      Vec<Subscription>,
}

impl Form {
    fn contribution(&self, cx: &App) -> knot_library::Result<Contribution> {
        let title = self.title.read(cx).value();
        let description = self.description.read(cx).value();
        let tags = self.tags.read(cx).value();
        Contribution::new(&Draft { kind:        self.submission.kind,
                                   id:          self.submission.id,
                                   title:       &title,
                                   description: &description,
                                   tags:        &tags,
                                   body:        &self.submission.body, })
    }

    /// The problem to show: a long description says so once typed; an
    /// empty one does not.
    fn problem(&self, error: &LibraryError, cx: &App) -> Option<String> {
        if matches!(error, LibraryError::InvalidDescription(_))
           && !self.description.read(cx).value().trim().is_empty()
        {
            return Some(t_with("library.submit.invalid_description",
                               &[("max", &DESCRIPTION_MAX.to_string())]));
        }
        problem(error)
    }
}

/// Asks for the description and tags `submission` needs, then opens it on
/// GitHub. The title starts as the item's name.
pub(crate) fn open_submit_dialog(submission: Submission, window: &mut Window, cx: &mut App) {
    let input = |placeholder: Option<&str>, value: String, window: &mut Window, cx: &mut App| {
        cx.new(|cx| {
              let mut state = InputState::new(window, cx);
              if let Some(placeholder) = placeholder {
                  state = state.placeholder(t(placeholder));
              }
              state.set_value(value, window, cx);
              state
          })
    };
    let title = input(None, submission.name.clone(), window, cx);
    let description = input(Some("library.submit.description_placeholder"),
                            String::new(),
                            window,
                            cx);
    let tags = input(Some("library.submit.tags_placeholder"),
                     String::new(),
                     window,
                     cx);
    let edits = [&title, &description, &tags].map(|input| {
                    window.subscribe(input, cx, |_, event: &InputEvent, window, _| {
                              if matches!(event, InputEvent::Change) {
                                  window.refresh();
                              }
                          })
                });
    let form = Rc::new(Form { submission,
                              title,
                              description: description.clone(),
                              tags,
                              _edits: edits.into() });
    window.open_dialog(cx, move |dialog, _window, app| {
              let checked = form.contribution(app);
              let problem = checked.as_ref()
                                   .err()
                                   .and_then(|error| form.problem(error, app));
              dialog.w(px(480.0))
                    .close_button(false)
                    .overlay_closable(false)
                    .title(t("library.submit.title"))
                    .content({
                        let form = Rc::clone(&form);
                        move |content, _window, app| {
                            content.child(body(&form, problem.clone(), app))
                        }
                    })
                    .footer(footer(checked.is_err()))
                    .on_ok({
                        let form = Rc::clone(&form);
                        move |_, window, app| confirm(&form, window, app)
                    })
                    .on_cancel(|_, window, app| {
                        window.dispatch_action(Box::new(Cancel), app);
                        true
                    })
          });
    description.update(cx, |input, cx| input.focus(window, cx));
}

fn body(form: &Form, problem: Option<String>, cx: &App) -> impl IntoElement {
    let muted = cx.theme().muted_foreground;
    // Labels right-aligned in a fixed column, as the settings rows are.
    let row = |key: &str, input: &Entity<InputState>| {
        h_flex().gap_3()
                .items_center()
                .child(div().w(px(90.0))
                            .flex_shrink_0()
                            .text_right()
                            .text_sm()
                            .child(t(key)))
                .child(div().flex_1().min_w_0().child(Input::new(input)))
    };
    v_flex().gap_3()
            .child(div().text_sm()
                        .text_color(muted)
                        .child(t("library.submit.intro")))
            .child(row("library.submit.title_label", &form.title))
            .child(row("library.submit.description_label", &form.description))
            .child(row("library.submit.tags_label", &form.tags))
            .children(problem.map(|problem| {
                                 div().text_sm().text_color(cx.theme().danger).child(problem)
                             }))
}

fn footer(disabled: bool) -> impl IntoElement {
    DialogFooter::new()
        .child(Button::new("library-submit-cancel").label(t("library.submit.cancel"))
                                                   .on_click(|_: &ClickEvent, window, app| {
                                                       window.dispatch_action(Box::new(Cancel),
                                                                              app);
                                                   }))
        .child(Button::new("library-submit-confirm").label(t("library.submit.submit"))
                                                    .primary()
                                                    .disabled(disabled)
                                                    .on_click(|_: &ClickEvent, window, app| {
                                                        window.dispatch_action(Box::new(Confirm { secondary: false }), app);
                                                    }))
}

/// Opens the checked item on GitHub and says how that went. Keeps the
/// dialog open while the form fails its check.
fn confirm(form: &Form, window: &mut Window, cx: &mut App) -> bool {
    let Ok(item) = form.contribution(cx)
    else {
        return false;
    };
    let outcome = submit(&item, crate::open_in::open_url, |file| {
        cx.write_to_clipboard(ClipboardItem::new_string(file));
    });
    window.push_notification(outcome.notification(), cx);
    true
}

#[cfg(test)]
mod tests;
