use uuid::Uuid;

use super::*;

const ID: &str = "399d4053-a1e4-4266-a847-de5afadb4caa";

fn draft<'a>(title: &'a str, description: &'a str, tags: &'a str, body: &'a str) -> Draft<'a> {
    Draft { kind: ContributionKind::Prompt,
            id: Uuid::parse_str(ID).unwrap(),
            title,
            description,
            tags,
            body }
}

#[test]
fn a_prompt_is_written_as_a_library_file() {
    let item = Contribution::new(&draft(" Code review ",
                                        "Review the changes on {{branch}}.",
                                        "Review, quality",
                                        "\nReview {{branch}}.\n\n")).unwrap();

    assert_eq!(item.path(), "prompts/code-review.md");
    assert_eq!(item.markdown(),
               format!("---\nid: {ID}\ntitle: \"Code review\"\ndescription: \"Review the \
                        changes on {{{{branch}}}}.\"\ntags: [review, quality]\n---\nReview \
                        {{{{branch}}}}.\n"));
}

#[test]
fn a_persona_goes_in_the_personas_directory_without_tags() {
    let mut persona = draft("Rust Pro", "A senior Rust developer.", "", "Be careful.");
    persona.kind = ContributionKind::Persona;
    let item = Contribution::new(&persona).unwrap();

    assert_eq!(item.path(), "personas/rust-pro.md");
    assert!(!item.markdown().contains("tags:"));
}

#[test]
fn yaml_significant_text_is_quoted() {
    let item = Contribution::new(&draft("Fix: \"the\" bug", "# not a comment", "", "x")).unwrap();

    assert!(item.markdown()
                .contains("title: \"Fix: \\\"the\\\" bug\"\ndescription: \"# not a comment\"\n"));
}

#[test]
fn slugs_keep_ascii_letters_and_digits_only() {
    assert_eq!(slug("  Hello, World!  2 "), "hello-world-2");
    assert_eq!(slug("Ünïcode & emoji 🎉"), "n-code-emoji");
    assert_eq!(slug("🎉"), "");
    assert_eq!(slug(&format!("{}-tail", "a".repeat(63))), "a".repeat(63));
}

#[test]
fn a_title_with_no_slug_is_named_for_the_id() {
    let item = Contribution::new(&draft("🎉", "Party.", "", "x")).unwrap();

    assert_eq!(item.file_name(), "399d4053.md");
}

#[test]
fn title_and_description_must_be_one_short_line() {
    assert!(matches!(Contribution::new(&draft("", "d", "", "x")),
                     Err(LibraryError::InvalidTitle(TITLE_MAX))));
    assert!(matches!(Contribution::new(&draft(&"t".repeat(81), "d", "", "x")),
                     Err(LibraryError::InvalidTitle(_))));
    assert!(matches!(Contribution::new(&draft("t", "two\nlines", "", "x")),
                     Err(LibraryError::InvalidDescription(DESCRIPTION_MAX))));
    assert!(matches!(Contribution::new(&draft("t", &"d".repeat(201), "", "x")),
                     Err(LibraryError::InvalidDescription(_))));
    assert!(Contribution::new(&draft(&"t".repeat(80), &"d".repeat(200), "", "x")).is_ok());
}

#[test]
fn tags_are_lowercased_deduplicated_and_checked() {
    let item = Contribution::new(&draft("t", "d", "Go,go  rust", "x")).unwrap();
    assert!(item.markdown().contains("tags: [go, rust]\n"));

    assert!(matches!(Contribution::new(&draft("t", "d", "c++", "x")),
                     Err(LibraryError::InvalidTag(tag)) if tag == "c++"));
    let eleven = (0..11).map(|n| format!("t{n}"))
                        .collect::<Vec<_>>()
                        .join(",");
    assert!(matches!(Contribution::new(&draft("t", "d", &eleven, "x")),
                     Err(LibraryError::TooManyTags(TAGS_MAX))));
}

#[test]
fn the_body_must_not_be_blank() {
    assert!(matches!(Contribution::new(&draft("t", "d", "", " \n ")),
                     Err(LibraryError::EmptyBody)));
}
