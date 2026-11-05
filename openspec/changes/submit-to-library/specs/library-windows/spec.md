## ADDED Requirements

### Requirement: A persona or prompt can be submitted to Knot-Library

Each row in the Personas and Prompts windows SHALL offer to submit its item
to Knot-Library. Submitting SHALL ask for a title, starting as the item's
name, a one-line description and optional tags, and SHALL NOT proceed until
they meet Knot-Library's rules: a title of one line and at most 80
characters, a description of one line and at most 200 characters, and at
most 10 tags of `a-z`, `0-9` and `-`, each at most 32 characters.

Proceeding SHALL open Knot-Library's new-file page in the browser at
`<kind-dir>/<slug>.md`, carrying the file: front matter with the item's own
id, the title, the description and any tags, followed by the prompt's text
or the persona's instructions. Knot SHALL NOT itself write to the library;
the pull request is proposed on GitHub. A file too long to carry in the
page's address SHALL be put on the clipboard and the page opened without it,
and the user SHALL be told which happened.

#### Scenario: A prompt opens as a new library file

- **WHEN** the user submits the prompt "Code review" with a description
- **THEN** Knot-Library's new-file page opens at `prompts/code-review.md`
  with the prompt as the file's body and the prompt's id in its front matter

#### Scenario: The form is checked before anything opens

- **WHEN** the description is empty, or a tag contains a character other
  than `a-z`, `0-9` and `-`
- **THEN** the submit button is disabled, and a bad tag is named

#### Scenario: A long file goes to the clipboard

- **WHEN** the file is too long to carry in the page's address
- **THEN** the file is on the clipboard, the new-file page opens empty, and
  the user is told to paste it
