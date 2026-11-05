//! An item written out as a Knot-Library contribution: the file a pull
//! request to the library adds, and where in the library it goes (#43).
//!
//! The rules are Knot-Library's `CONTRIBUTING.md`: one file per item,
//! `<kind-dir>/<slug>.md`, front matter of `id`, `title`, `description` and
//! optionally `tags`, then a non-empty body. Checking them here, before the
//! user leaves for GitHub, saves a round trip through the library's own
//! Check workflow for the mistakes Knot can see.
//!
//! An item keeps its own id, so importing it back later recognises it as
//! held. Library ids are permanent, so an item whose id the library already
//! has - one imported from it, edited since - is an update to that file
//! ([`Contribution::updating`]), not a new item under a new name.

use uuid::Uuid;

use crate::error::{LibraryError, Result};
use crate::index::ItemKind;

/// The longest title the library accepts, in characters.
pub const TITLE_MAX: usize = 80;
/// The longest description the library accepts, in characters.
pub const DESCRIPTION_MAX: usize = 200;
/// The most tags an item may carry.
pub const TAGS_MAX: usize = 10;
/// The longest tag, and the longest slug's share of a file name.
const TAG_MAX: usize = 32;
const SLUG_MAX: usize = 64;

/// The kinds a contribution can be. The library has others only as an
/// index can name them (`ItemKind::Other`), which nothing can submit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionKind {
    Persona,
    Prompt,
}

impl ContributionKind {
    /// The library directory items of this kind live in.
    pub fn directory(self) -> &'static str {
        match self {
            Self::Persona => "personas",
            Self::Prompt => "prompts",
        }
    }
}

impl From<ContributionKind> for ItemKind {
    fn from(kind: ContributionKind) -> Self {
        match kind {
            ContributionKind::Persona => Self::Persona,
            ContributionKind::Prompt => Self::Prompt,
        }
    }
}

/// A checked contribution, ready to be written out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contribution {
    kind:        ContributionKind,
    id:          Uuid,
    title:       String,
    description: String,
    tags:        Vec<String>,
    body:        String,
    /// The library's path for this id, when it already has the item.
    existing:    Option<String>,
}

/// What [`Contribution::new`] is built from, before checking.
#[derive(Debug, Clone)]
pub struct Draft<'a> {
    pub kind:        ContributionKind,
    pub id:          Uuid,
    pub title:       &'a str,
    pub description: &'a str,
    /// Comma- or space-separated, as typed.
    pub tags:        &'a str,
    pub body:        &'a str,
}

impl Contribution {
    /// Checks `draft` against the library's rules. Title, description and
    /// tags are trimmed first, and tags lowercased, so what a user would
    /// read as the same value is accepted as it.
    pub fn new(draft: &Draft<'_>) -> Result<Self> {
        let title = one_line(draft.title, TITLE_MAX, LibraryError::InvalidTitle)?;
        let description = one_line(draft.description,
                                   DESCRIPTION_MAX,
                                   LibraryError::InvalidDescription)?;
        let tags = parse_tags(draft.tags)?;
        let body = draft.body.trim();
        if body.is_empty() {
            return Err(LibraryError::EmptyBody);
        }
        Ok(Self { kind: draft.kind,
                  id: draft.id,
                  title,
                  description,
                  tags,
                  body: body.to_owned(),
                  existing: None })
    }

    pub fn kind(&self) -> ContributionKind {
        self.kind
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    /// This contribution as an update to the library's file at `path`,
    /// which already holds this id.
    #[must_use]
    pub fn updating(mut self, path: impl Into<String>) -> Self {
        self.existing = Some(path.into());
        self
    }

    /// Whether this updates an item the library already has.
    pub fn is_update(&self) -> bool {
        self.existing.is_some()
    }

    /// The file's name: the existing file's for an update; otherwise the
    /// title as a slug, or the id's first group when the title has nothing
    /// a slug can keep (a title of emoji, say).
    pub fn file_name(&self) -> String {
        if let Some(path) = &self.existing {
            return path.rsplit('/').next().unwrap_or(path).to_owned();
        }
        let slug = slug(&self.title);
        if slug.is_empty() {
            let id = self.id.to_string();
            let short = id.split('-').next().unwrap_or(&id);
            return format!("{short}.md");
        }
        format!("{slug}.md")
    }

    /// The file's path in the library: the existing one for an update,
    /// otherwise `<kind-dir>/<slug>.md`.
    pub fn path(&self) -> String {
        if let Some(path) = &self.existing {
            return path.clone();
        }
        format!("{}/{}", self.kind.directory(), self.file_name())
    }

    /// The file itself: front matter, then the body.
    ///
    /// Title and description are written as double-quoted scalars, so a `:`,
    /// a leading `#` or `[` and the like read as text rather than YAML. A
    /// JSON string is a valid YAML double-quoted scalar.
    pub fn markdown(&self) -> String {
        let mut text = format!("---\nid: {}\ntitle: {}\ndescription: {}\n",
                               self.id,
                               quoted(&self.title),
                               quoted(&self.description));
        if !self.tags.is_empty() {
            text.push_str(&format!("tags: [{}]\n", self.tags.join(", ")));
        }
        text.push_str("---\n");
        text.push_str(&self.body);
        text.push('\n');
        text
    }
}

fn quoted(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("\"{value}\""))
}

/// `value` trimmed, if it is one non-empty line of at most `max` characters.
fn one_line(value: &str, max: usize, error: fn(usize) -> LibraryError) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.contains(['\n', '\r']) || value.chars().count() > max {
        return Err(error(max));
    }
    Ok(value.to_owned())
}

fn parse_tags(tags: &str) -> Result<Vec<String>> {
    let mut parsed: Vec<String> = Vec::new();
    for tag in tags.split([',', ' ', '\n'])
                   .map(str::trim)
                   .filter(|tag| !tag.is_empty())
    {
        let tag = tag.to_lowercase();
        let valid =
            tag.len() <= TAG_MAX
            && tag.bytes()
                  .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if !valid {
            return Err(LibraryError::InvalidTag(tag));
        }
        if !parsed.contains(&tag) {
            parsed.push(tag);
        }
    }
    if parsed.len() > TAGS_MAX {
        return Err(LibraryError::TooManyTags(TAGS_MAX));
    }
    Ok(parsed)
}

/// `title` as a library slug: lowercase ASCII letters and digits, runs of
/// anything else as one `-`, at most 64 characters, starting and ending
/// alphanumeric. Empty when the title has no ASCII letter or digit.
pub fn slug(title: &str) -> String {
    let mut slug = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        }
        else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.truncate(SLUG_MAX);
    slug.trim_end_matches('-').to_owned()
}

#[cfg(test)]
#[path = "contribution/tests.rs"]
mod tests;
