//! Submitting a persona or prompt to Knot-Library (#43), in two steps.
//!
//! Contract: `openspec/specs/library-windows/spec.md`, "A persona or prompt
//! can be submitted to Knot-Library".
//!
//! **Details** asks for what the library needs that Knot does not keep - a
//! title, a one-line description, optional tags - while the library's index
//! is read to learn whether it already has the item's id. **Review** then
//! shows the exact file, editable, and nothing leaves Knot until the user
//! confirms it: a prompt saved from a conversation holds the text as sent,
//! and can carry expanded local values - paths, branch names, host names.
//!
//! GitHub does the rest: its new-file page forks the library for someone who
//! cannot write to it, and "Propose changes" opens the pull request. So
//! nothing here needs the GitHub CLI or a token. The file's rules are
//! `knot_library::Contribution`'s; where it goes is `publish`'s.

mod publish;

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::AnyElement;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::ClickEvent;
use gpui_kit::ClipboardItem;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
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
use gpui_kit::component::input::Textarea;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::notification::Notification;
use gpui_kit::div;
use gpui_kit::px;
use knot_core::consts::{KNOT_LIBRARY_BRANCH, KNOT_LIBRARY_REPO};
use knot_core::l10n::{t, t_with};
use knot_library::contribution::{DESCRIPTION_MAX, TAGS_MAX, TITLE_MAX};
use knot_library::{Contribution, ContributionKind, Draft, LibraryError, Location};
use publish::{InLibrary, Outcome, Publication};
use uuid::Uuid;

/// The persona or prompt a row offers for submission.
#[derive(Debug, Clone)]
pub(crate) struct Submission {
    pub(crate) kind:        ContributionKind,
    pub(crate) id:          Uuid,
    pub(crate) name:        String,
    /// A prompt's text, or a persona's instructions.
    pub(crate) body:        String,
    /// The description the item already has, if any - a prompt's.
    pub(crate) description: Option<String>,
}

impl Outcome {
    fn notification(self) -> Notification {
        match self {
            Self::Opened => Notification::info(t("library.submit.opened")),
            Self::OpenedCopied => Notification::info(t("library.submit.opened_copied")),
            Self::UpdateCopied => Notification::info(t("library.submit.update_copied")),
            Self::OpenFailed => Notification::error(t("library.submit.open_failed")),
        }
    }
}

/// What to tell the user about a draft the library would refuse, or `None`
/// while there is nothing to say yet - before a description is typed, the
/// disabled button says enough.
fn problem(error: &LibraryError) -> Option<String> {
    match error {
        LibraryError::InvalidTitle(_) => Some(t_with("library.submit.invalid_title",
                                                     &[("max", &TITLE_MAX.to_string())])),
        LibraryError::InvalidTag(tag) => {
            Some(t_with("library.submit.invalid_tag", &[("tag", tag)]))
        }
        LibraryError::TooManyTags(_) => Some(t_with("library.submit.too_many_tags",
                                                    &[("max", &TAGS_MAX.to_string())])),
        _ => None,
    }
}

/// Which step the dialog is on.
enum Step {
    Details,
    /// The file being reviewed, and the item it was written from.
    Review {
        item: Contribution,
        text: Entity<TextareaState>,
    },
}

/// The dialog's state, shared by its builder and its handlers.
struct Form {
    submission:  Submission,
    title:       Entity<InputState>,
    description: Entity<InputState>,
    tags:        Entity<InputState>,
    step:        RefCell<Step>,
    in_library:  Rc<RefCell<InLibrary>>,
    /// Redraw the window on each edit, so the buttons' enabled state and
    /// the problem line follow the typing. Dropped with the dialog.
    edits:       RefCell<Vec<Subscription>>,
}

impl Form {
    fn contribution(&self, cx: &App) -> knot_library::Result<Contribution> {
        let title = self.title.read(cx).value();
        let description = self.description.read(cx).value();
        let tags = self.tags.read(cx).value();
        let item = Contribution::new(&Draft { kind:        self.submission.kind,
                                              id:          self.submission.id,
                                              title:       &title,
                                              description: &description,
                                              tags:        &tags,
                                              body:        &self.submission.body, })?;
        Ok(match &*self.in_library.borrow() {
            InLibrary::Update(path) => item.updating(path.clone()),
            InLibrary::Checking | InLibrary::New | InLibrary::Unknown => item,
        })
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

    fn watch<T: gpui_kit::EventEmitter<InputEvent>>(&self, input: &Entity<T>,
                                                    window: &mut Window, cx: &mut App) {
        let edit = window.subscribe(input, cx, |_, event: &InputEvent, window, _| {
                             if matches!(event, InputEvent::Change) {
                                 window.refresh();
                             }
                         });
        self.edits.borrow_mut().push(edit);
    }
}

/// Asks for the details `submission` needs, shows the file for review, then
/// opens it on GitHub. The title starts as the item's name.
pub(crate) fn open_submit_dialog(submission: Submission, window: &mut Window, cx: &mut App) {
    let input = |placeholder: Option<&str>, value: String, window: &mut Window, cx: &mut App| {
        cx.new(|cx| {
              let mut state = InputState::new(window, cx);
              if let Some(placeholder) = placeholder {
                  state = state.placeholder(t(placeholder));
              }
              state.default_value(value)
          })
    };
    let title = input(None, submission.name.clone(), window, cx);
    let description = input(Some("library.submit.description_placeholder"),
                            submission.description.clone().unwrap_or_default(),
                            window,
                            cx);
    let tags = input(Some("library.submit.tags_placeholder"),
                     String::new(),
                     window,
                     cx);
    let in_library = Rc::new(RefCell::new(InLibrary::Checking));
    look_up(&submission, Rc::clone(&in_library), window, cx);
    let form = Rc::new(Form { submission,
                              title,
                              description: description.clone(),
                              tags,
                              step: RefCell::new(Step::Details),
                              in_library,
                              edits: RefCell::default() });
    for input in [&form.title, &form.description, &form.tags] {
        form.watch(input, window, cx);
    }
    window.open_dialog(cx, move |dialog, _window, app| {
              let footer = footer(&form, app);
              dialog.w(px(560.0))
                    .close_button(false)
                    .overlay_closable(false)
                    .title(t("library.submit.title"))
                    .content({
                        let form = Rc::clone(&form);
                        move |content, _, app| content.child(body(&form, app))
                    })
                    .footer(footer)
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

/// Where the lookup reads the library's index from, when not Knot-Library
/// itself - set by tests, so opening the dialog reaches no network.
#[cfg(test)]
pub(crate) struct LookupLocation(pub(crate) Location);

#[cfg(test)]
impl gpui_kit::Global for LookupLocation {}

fn lookup_location(cx: &App) -> Location {
    #[cfg(test)]
    if let Some(LookupLocation(location)) = cx.try_global::<LookupLocation>() {
        return location.clone();
    }
    let _ = cx;
    Location::GitHub { repo:   KNOT_LIBRARY_REPO.to_owned(),
                       branch: Some(KNOT_LIBRARY_BRANCH.to_owned()), }
}

/// Reads Knot-Library's index off the main thread, to learn whether it
/// already has `submission`'s id, and redraws with the answer.
fn look_up(submission: &Submission, in_library: Rc<RefCell<InLibrary>>, window: &Window,
           cx: &mut App) {
    let (id, kind) = (submission.id, submission.kind);
    let location = lookup_location(cx);
    window.spawn(cx, async move |cx| {
              let index =
            cx.background_executor()
              .spawn(async move { knot_library::fetcher_for(&location).fetch_index(&location) })
              .await;
              *in_library.borrow_mut() = match index {
                  Ok(index) => InLibrary::of(&index, id, kind),
                  Err(_) => InLibrary::Unknown,
              };
              let _ = cx.update(|window, _| window.refresh());
          })
          .detach();
}

/// The showing step's body.
fn body(form: &Form, cx: &App) -> AnyElement {
    match &*form.step.borrow() {
        Step::Details => details(form, cx),
        Step::Review { text, .. } => review(text, cx),
    }
}

/// The details step: the fields, whether the library has the item already,
/// and what is wrong with the draft, if anything.
fn details(form: &Form, cx: &App) -> AnyElement {
    let problem = form.contribution(cx)
                      .err()
                      .and_then(|error| form.problem(&error, cx));
    let muted = cx.theme().muted_foreground;
    let in_library = match &*form.in_library.borrow() {
        InLibrary::Checking => Some(t("library.submit.checking")),
        InLibrary::New => None,
        InLibrary::Update(path) => Some(t_with("library.submit.updates", &[("path", path)])),
        InLibrary::Unknown => Some(t("library.submit.unchecked")),
    };
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
            .children(in_library.map(|note| div().text_sm().text_color(muted).child(note)))
            .children(problem.map(|problem| {
                                 div().text_sm().text_color(cx.theme().danger).child(problem)
                             }))
            .into_any_element()
}

/// The review step: the file, editable, with a warning when it holds
/// something that looks local or private.
fn review(text: &Entity<TextareaState>, cx: &App) -> AnyElement {
    let value = text.read(cx).value();
    let flagged = publish::has_local_values(&value, crate::bug_report::home_dir().as_deref());
    v_flex().gap_3()
            .child(div().text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t("library.submit.review_intro")))
            .children(flagged.then(|| {
                                 div().text_sm()
                                      .text_color(cx.theme().danger)
                                      .child(t("library.submit.local_values"))
                             }))
            .child(div().debug_selector(|| "library-submit-review".into())
                        .h(px(320.0))
                        .child(Textarea::new(text).h_full()))
            .into_any_element()
}

/// Cancel and Review on the details step; Back and Open on GitHub on the
/// review step. Each primary action stays disabled until it can proceed.
fn footer(form: &Rc<Form>, cx: &App) -> AnyElement {
    let (secondary, primary, disabled) = match &*form.step.borrow() {
        Step::Details => {
            let checking = matches!(*form.in_library.borrow(), InLibrary::Checking);
            (Button::new("library-submit-cancel").label(t("library.submit.cancel"))
                                                 .on_click(|_: &ClickEvent, window, app| {
                                                     window.dispatch_action(Box::new(Cancel), app);
                                                 }),
             Button::new("library-submit-review").label(t("library.submit.review")),
             checking || form.contribution(cx).is_err())
        }
        Step::Review { text, .. } => {
            let form = Rc::clone(form);
            (Button::new("library-submit-back").label(t("library.submit.back"))
                                               .on_click(move |_: &ClickEvent, window, _| {
                                                   *form.step.borrow_mut() = Step::Details;
                                                   window.refresh();
                                               }),
             Button::new("library-submit-confirm").label(t("library.submit.submit")),
             text.read(cx).value().trim().is_empty())
        }
    };
    DialogFooter::new().child(secondary)
                       .child(primary.primary()
                                     .disabled(disabled)
                                     .on_click(|_: &ClickEvent, window, app| {
                                         window.dispatch_action(Box::new(Confirm { secondary: false }), app);
                                     }))
                       .into_any_element()
}

/// The primary action of whichever step is showing. From details it moves
/// to review, keeping the dialog open; from review it publishes and closes.
fn confirm(form: &Form, window: &mut Window, cx: &mut App) -> bool {
    let step = std::mem::replace(&mut *form.step.borrow_mut(), Step::Details);
    match step {
        Step::Details => {
            if matches!(*form.in_library.borrow(), InLibrary::Checking) {
                return false;
            }
            let Ok(item) = form.contribution(cx)
            else {
                return false;
            };
            let file = item.markdown();
            let text = cx.new(|cx| TextareaState::new(window, cx).default_value(file));
            form.watch(&text, window, cx);
            *form.step.borrow_mut() = Step::Review { item, text };
            window.refresh();
            false
        }
        Step::Review { item, text } => {
            let file = text.read(cx).value().to_string();
            if file.trim().is_empty() {
                *form.step.borrow_mut() = Step::Review { item, text };
                return false;
            }
            keep_description(form, cx);
            let publication = Publication::new(&item, file);
            let outcome = publish::publish(&publication, crate::open_in::open_url, |file| {
                cx.write_to_clipboard(ClipboardItem::new_string(file));
            });
            window.push_notification(outcome.notification(), cx);
            true
        }
    }
}

/// A prompt keeps the description it was submitted with, so the next
/// submission starts from it.
fn keep_description(form: &Form, cx: &mut App) {
    if form.submission.kind != ContributionKind::Prompt {
        return;
    }
    let id = form.submission.id;
    let description = form.description.read(cx).value().to_string();
    if let Err(error) = crate::settings_global::write_persisting(cx, |settings| {
        settings.set_prompt_description(id, &description)
    }) {
        eprintln!("failed to keep the prompt's description: {error}");
    }
}

#[cfg(test)]
mod tests;
