//! The review flow's pure parts: where it starts, the reply it begins, and
//! the font size steps' bounds.

use super::ReviewState;
use super::review_preamble;
use super::stepped_font_size;
use crate::consts;

#[test]
fn a_file_starts_out_being_read() {
    assert_eq!(ReviewState::default(), ReviewState::Viewing);
}

/// The agent may have several artifacts in flight; the reply has to say
/// which one the comments are about.
#[test]
fn the_review_preamble_names_the_file_and_leaves_room_for_comments() {
    let preamble = review_preamble("plan.md");

    assert!(preamble.contains("plan.md"), "{preamble}");
    assert!(preamble.ends_with('\n'),
            "the comments go on the line after the preamble");
}

#[test]
fn the_font_size_steps_within_its_bounds() {
    assert_eq!(stepped_font_size(14, 1), Some(15));
    assert_eq!(stepped_font_size(14, -1), Some(13));
    assert_eq!(stepped_font_size(consts::MARKDOWN_FONT_SIZE_MAX, 1), None);
    assert_eq!(stepped_font_size(consts::MARKDOWN_FONT_SIZE_MIN, -1), None);
    assert_eq!(stepped_font_size(consts::MARKDOWN_FONT_SIZE_MAX, -1),
               Some(consts::MARKDOWN_FONT_SIZE_MAX - 1));
}

/// A stored size outside the bounds - a hand-edited settings file - still
/// steps back toward them rather than being stuck.
#[test]
fn an_out_of_range_size_can_step_back_in() {
    assert_eq!(stepped_font_size(consts::MARKDOWN_FONT_SIZE_MAX + 1, -1),
               Some(consts::MARKDOWN_FONT_SIZE_MAX));
}

/// The approval names the file, so it is not mistaken for agreement to
/// something else pending (#30).
#[test]
fn an_approval_names_the_file() {
    assert_eq!(super::approval("reports/cost-report.md", None),
               "Approved: reports/cost-report.md");
}

#[test]
fn an_approval_carries_the_review_comments() {
    let text = super::approval("plan.md", Some("  keep step 2 short\n"));
    assert_eq!(text,
               "Approved: plan.md, with these review comments:\nkeep step 2 short");
    assert_eq!(super::approval("plan.md", Some("   ")),
               "Approved: plan.md",
               "blank comments are no comments");
}

/// A file inside the agent's folder is named as the agent would name it.
#[test]
fn the_approved_file_is_named_relative_to_the_agent_folder() {
    use std::path::Path;
    let file = Path::new("/work/repo/cost-analysis/reports/2026-10-01/cost-report.md");
    assert_eq!(super::approval_target(file, "/work/repo"),
               "cost-analysis/reports/2026-10-01/cost-report.md");
    assert_eq!(super::approval_target(file, "/elsewhere"),
               file.to_string_lossy(),
               "outside the folder, in full");
}

/// Mid-review, the comments are what follows the preamble; a draft the user
/// had before the review stays theirs.
#[test]
fn a_started_review_splits_into_draft_and_comments() {
    let preamble = super::review_preamble("plan.md");
    let text = format!("my earlier draft\n{preamble}tighten the rollout\n");
    assert_eq!(super::split_review(&text, &preamble),
               ("my earlier draft".to_owned(), "tighten the rollout".to_owned()));
    assert_eq!(super::split_review("just comments", &preamble),
               (String::new(), "just comments".to_owned()));
}
