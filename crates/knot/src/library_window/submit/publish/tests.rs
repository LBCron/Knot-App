use std::cell::RefCell;

use knot_library::{Contribution, ContributionKind, Draft, Index, IndexItem, ItemKind};
use uuid::Uuid;

use super::*;

fn item(id: Uuid) -> Contribution {
    Contribution::new(&Draft { kind: ContributionKind::Prompt,
                               id,
                               title: "Code review",
                               description: "Review the branch.",
                               tags: "",
                               body: "Review it." }).unwrap()
}

fn publication(text: &str) -> Publication {
    Publication::new(&item(Uuid::new_v4()), text.to_owned())
}

fn index_with(kind: ItemKind, id: Uuid) -> Index {
    Index { format:       1,
            commit:       "abc".into(),
            generated_at: String::new(),
            kinds:        Default::default(),
            items:        vec![IndexItem { kind,
                                           id: id.to_string(),
                                           slug: "code-review".into(),
                                           title: "Code review".into(),
                                           description: "d".into(),
                                           tags: vec![],
                                           authors: vec![],
                                           path: "prompts/code-review.md".into(),
                                           size: 0,
                                           sha256: String::new() }], }
}

/// Records what `publish` opened and copied.
fn run(publication: &Publication, opens: bool) -> (Outcome, Vec<String>, Option<String>) {
    let opened = RefCell::new(Vec::new());
    let copied = RefCell::new(None);
    let outcome = publish(publication,
                          |url| {
                              opened.borrow_mut().push(url.to_owned());
                              opens
                          },
                          |file| *copied.borrow_mut() = Some(file));
    (outcome, opened.into_inner(), copied.into_inner())
}

#[test]
fn an_id_the_library_has_is_an_update_to_its_file() {
    let id = Uuid::new_v4();
    assert_eq!(InLibrary::of(&index_with(ItemKind::Prompt, id),
                             id,
                             ContributionKind::Prompt),
               InLibrary::Update("prompts/code-review.md".into()));
    assert_eq!(InLibrary::of(&index_with(ItemKind::Prompt, Uuid::new_v4()),
                             id,
                             ContributionKind::Prompt),
               InLibrary::New);
    assert_eq!(InLibrary::of(&index_with(ItemKind::Persona, id),
                             id,
                             ContributionKind::Prompt),
               InLibrary::New,
               "an id is matched within its kind");
}

#[test]
fn a_new_item_opens_the_new_file_page_with_the_reviewed_text() {
    let publication = publication("---\nedited\n---\nby hand\n");
    let (outcome, opened, copied) = run(&publication, true);

    assert_eq!(outcome, Outcome::Opened);
    assert_eq!(opened.len(), 1);
    assert!(opened[0].starts_with(&format!("https://github.com/{KNOT_LIBRARY_REPO}/new/\
                                            {KNOT_LIBRARY_BRANCH}/prompts?filename=code-review.md\
                                            &value=")),
            "{}",
            opened[0]);
    assert!(opened[0].ends_with(&percent_encode("---\nedited\n---\nby hand\n")),
            "the reviewed text is what is sent, not the generated one");
    assert_eq!(copied, None);
}

#[test]
fn an_update_opens_the_existing_file_for_editing_with_the_text_on_the_clipboard() {
    let id = Uuid::new_v4();
    let publication = Publication::new(&item(id).updating("prompts/review.md"), "file".into());
    let (outcome, opened, copied) = run(&publication, true);

    assert_eq!(outcome, Outcome::UpdateCopied);
    assert_eq!(opened,
               [format!("https://github.com/{KNOT_LIBRARY_REPO}/edit/{KNOT_LIBRARY_BRANCH}/\
                         prompts/review.md")]);
    assert_eq!(copied.as_deref(), Some("file"));
}

#[test]
fn a_long_file_goes_to_the_clipboard_and_the_page_opens_empty() {
    let text = "word ".repeat(LIBRARY_SUBMIT_URL_MAX);
    let publication = publication(&text);
    let (outcome, opened, copied) = run(&publication, true);

    assert_eq!(outcome, Outcome::OpenedCopied);
    assert_eq!(opened, [publication.new_file_url(false)]);
    assert_eq!(copied, Some(text));
}

#[test]
fn no_browser_leaves_the_file_on_the_clipboard() {
    let (outcome, _, copied) = run(&publication("short"), false);

    assert_eq!(outcome, Outcome::OpenFailed);
    assert_eq!(copied.as_deref(), Some("short"));
}

#[test]
fn local_values_are_flagged() {
    assert!(has_local_values("see /Users/paul/src/app", Some("/Users/paul")));
    assert!(has_local_values("mail me@example.com", None));
    assert!(has_local_values("token=ghp_abcdefghijklmnopqrstuvwxyz0123456789", None));
    assert!(!has_local_values("---\nid: 399d4053-a1e4-4266-a847-de5afadb4caa\n---\nReview \
                               {{branch}} in {{folder}}.\n",
                              Some("/Users/paul")));
}
