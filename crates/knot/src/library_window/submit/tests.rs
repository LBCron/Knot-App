use std::cell::RefCell;

use knot_library::{Contribution, ContributionKind, Draft, LibraryError};
use uuid::Uuid;

use super::*;

fn item(body: &str) -> Contribution {
    Contribution::new(&Draft { kind: ContributionKind::Prompt,
                               id: Uuid::new_v4(),
                               title: "Code review",
                               description: "Review the branch.",
                               tags: "",
                               body }).unwrap()
}

#[test]
fn the_new_file_page_is_in_the_library_at_the_items_path() {
    let item = item("Review {{branch}} & fix.");
    let url = new_file_url(&item, true);

    assert!(url.starts_with(&format!("https://github.com/{KNOT_LIBRARY_REPO}/new/\
                                      {KNOT_LIBRARY_BRANCH}/prompts?filename=code-review.md\
                                      &value=")),
            "{url}");
    assert!(url.ends_with(&percent_encode(&item.markdown())));
    assert!(!new_file_url(&item, false).contains("value="));
}

#[test]
fn a_short_item_opens_with_the_file_in_it() {
    let opened = RefCell::new(Vec::new());
    let copied = RefCell::new(None);
    let item = item("Short.");

    let outcome = submit(&item,
                         |url| {
                             opened.borrow_mut().push(url.to_owned());
                             true
                         },
                         |file| *copied.borrow_mut() = Some(file));

    assert_eq!(outcome, Outcome::Opened);
    assert_eq!(opened.into_inner(), [new_file_url(&item, true)]);
    assert_eq!(copied.into_inner(), None);
}

#[test]
fn a_long_item_goes_to_the_clipboard_and_the_page_opens_empty() {
    let opened = RefCell::new(Vec::new());
    let copied = RefCell::new(None);
    let item = item(&"word ".repeat(LIBRARY_SUBMIT_URL_MAX));

    let outcome = submit(&item,
                         |url| {
                             opened.borrow_mut().push(url.to_owned());
                             true
                         },
                         |file| *copied.borrow_mut() = Some(file));

    assert_eq!(outcome, Outcome::OpenedCopied);
    assert_eq!(opened.into_inner(), [new_file_url(&item, false)]);
    assert_eq!(copied.into_inner(), Some(item.markdown()));
}

#[test]
fn no_browser_leaves_the_file_on_the_clipboard() {
    let copied = RefCell::new(None);
    let item = item("Short.");

    let outcome = submit(&item, |_| false, |file| *copied.borrow_mut() = Some(file));

    assert_eq!(outcome, Outcome::OpenFailed);
    assert_eq!(copied.into_inner(), Some(item.markdown()));
}

#[test]
fn every_problem_and_outcome_resolves() {
    for key in ["library.submit.persona_tooltip",
                "library.submit.prompt_tooltip",
                "library.submit.title",
                "library.submit.intro",
                "library.submit.title_label",
                "library.submit.description_label",
                "library.submit.description_placeholder",
                "library.submit.tags_label",
                "library.submit.tags_placeholder",
                "library.submit.cancel",
                "library.submit.submit",
                "library.submit.invalid_description",
                "library.submit.opened",
                "library.submit.opened_copied",
                "library.submit.open_failed"]
    {
        assert_ne!(t(key), key, "{key} is missing from the catalogue");
    }
    let title = problem(&LibraryError::InvalidTitle(TITLE_MAX)).unwrap();
    assert!(title.contains(&TITLE_MAX.to_string()), "{title}");
    let tag = problem(&LibraryError::InvalidTag("c++".into())).unwrap();
    assert!(tag.contains("c++"), "{tag}");
    let many = problem(&LibraryError::TooManyTags(TAGS_MAX)).unwrap();
    assert!(many.contains(&TAGS_MAX.to_string()), "{many}");
    assert_eq!(problem(&LibraryError::InvalidDescription(DESCRIPTION_MAX)),
               None,
               "an empty description is left to the disabled button");
}
