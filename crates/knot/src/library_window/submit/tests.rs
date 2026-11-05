use knot_library::LibraryError;

use super::*;

#[test]
fn every_submit_label_resolves() {
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
                "library.submit.review",
                "library.submit.back",
                "library.submit.submit",
                "library.submit.checking",
                "library.submit.unchecked",
                "library.submit.review_intro",
                "library.submit.local_values",
                "library.submit.opened",
                "library.submit.opened_copied",
                "library.submit.update_copied",
                "library.submit.open_failed"]
    {
        assert_ne!(t(key), key, "{key} is missing from the catalogue");
    }
    let updates = t_with("library.submit.updates", &[("path", "prompts/x.md")]);
    assert!(updates.contains("prompts/x.md"), "{updates}");
    let long = t_with("library.submit.invalid_description",
                      &[("max", &DESCRIPTION_MAX.to_string())]);
    assert!(long.contains(&DESCRIPTION_MAX.to_string()), "{long}");
}

#[test]
fn each_refusal_says_what_to_fix() {
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
