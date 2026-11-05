# Proposal

## Why

Knot imports personas and prompts from Knot-Library but offers no way back:
sharing one meant hand-writing the library's front matter and opening a pull
request outside Knot (#43).

## What Changes

- Each row in the Personas and Prompts windows gets a "Submit to
  Knot-Library…" action.
- A dialog asks for what the library needs and Knot does not keep: a title
  (starting as the name), a one-line description and optional tags, checked
  against Knot-Library's contribution rules before anything opens.
- The item opens as a new file in Knot-Library on GitHub's new-file page.
  GitHub forks the library for a user who cannot write to it, and "Propose
  changes" opens the pull request. A file too long for a web address goes to
  the clipboard and the page opens empty.

## Decisions

- **The browser, not the GitHub CLI.** The new-file page handles the fork,
  the branch and the pull request for any signed-in user, and the user sees
  the exact file before it is proposed. Driving `gh repo fork` and `gh pr
  create` would need git operations against a fork Knot does not otherwise
  keep, for no gain in what is submitted.
- **The item keeps its id.** Importing the item back later then recognises
  it as held. The cost is that an item imported from the library carries an
  id the library already has; its own Check workflow rejects that pull
  request, which Knot cannot know without fetching the index.

## Impact

- `knot-library`: `contribution` - the file, its path and the library's
  rules for it.
- `knot`: `library_window::submit` (dialog and hand-off), the Personas and
  Prompts rows; `percent_encode` moves to `open_in` beside `open_url` so the
  bug report and this share it.
- Docs: `docs/site/latest/personas-prompts.md`.
