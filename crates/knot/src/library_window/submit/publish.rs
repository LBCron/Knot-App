//! Where a reviewed Knot-Library file goes, and how it gets there (#43):
//! the library's new-file page for a new item, its edit page for one it
//! already has, and the clipboard when the browser cannot carry the file.
//!
//! No GPUI here, so each decision is tested without a window.

use knot_core::consts::{KNOT_LIBRARY_BRANCH, KNOT_LIBRARY_REPO};
use knot_library::{Contribution, ContributionKind, Index, ItemKind};
use uuid::Uuid;

use crate::consts::LIBRARY_SUBMIT_URL_MAX;
use crate::open_in::percent_encode;

/// Whether Knot-Library already has an item's id, as its index says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InLibrary {
    /// The index is being fetched.
    Checking,
    /// The library does not have the id: a new item.
    New,
    /// The library has the id, at this path: an update to that file.
    Update(String),
    /// The index could not be read, so the item goes as new and the
    /// library's own check has the last word.
    Unknown,
}

impl InLibrary {
    /// What `index` says about item `id` of `kind`.
    pub(super) fn of(index: &Index, id: Uuid, kind: ContributionKind) -> Self {
        let kind = ItemKind::from(kind);
        index.items
             .iter()
             .find(|item| item.kind == kind && Uuid::parse_str(&item.id).ok() == Some(id))
             .map_or(Self::New, |item| Self::Update(item.path.clone()))
    }
}

/// A reviewed file, and where in the library it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Publication {
    pub(super) kind:      ContributionKind,
    pub(super) path:      String,
    pub(super) is_update: bool,
    /// The file as the user left it in the review step.
    pub(super) text:      String,
}

impl Publication {
    /// `item`'s place in the library, carrying the reviewed `text`.
    pub(super) fn new(item: &Contribution, text: String) -> Self {
        Self { kind: item.kind(),
               path: item.path(),
               is_update: item.is_update(),
               text }
    }

    fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// The library's new-file page for this file, carrying it when
    /// `with_file`.
    pub(super) fn new_file_url(&self, with_file: bool) -> String {
        let mut url = format!("https://github.com/{KNOT_LIBRARY_REPO}/new/{KNOT_LIBRARY_BRANCH}/{}\
                               ?filename={}",
                              self.kind.directory(),
                              percent_encode(self.file_name()));
        if with_file {
            url.push_str("&value=");
            url.push_str(&percent_encode(&self.text));
        }
        url
    }

    /// The library's edit page for the file this updates. GitHub's editor
    /// takes no content from the address, so the file goes by clipboard.
    pub(super) fn edit_url(&self) -> String {
        format!("https://github.com/{KNOT_LIBRARY_REPO}/edit/{KNOT_LIBRARY_BRANCH}/{}",
                self.path)
    }
}

/// How handing the file to GitHub went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    /// The new-file page opened with the file in it.
    Opened,
    /// The file was too long for an address: it is on the clipboard and the
    /// new-file page opened empty.
    OpenedCopied,
    /// The library's existing file opened for editing, with the new version
    /// on the clipboard to replace it.
    UpdateCopied,
    /// No browser opened; the file is on the clipboard.
    OpenFailed,
}

/// Opens `publication` on GitHub through `open_url`, putting the file on the
/// clipboard through `copy` wherever the page cannot carry it.
pub(super) fn publish(publication: &Publication, open_url: impl Fn(&str) -> bool,
                      copy: impl FnOnce(String))
                      -> Outcome {
    if publication.is_update {
        copy(publication.text.clone());
        return if open_url(&publication.edit_url()) {
            Outcome::UpdateCopied
        }
        else {
            Outcome::OpenFailed
        };
    }
    let url = publication.new_file_url(true);
    if url.len() <= LIBRARY_SUBMIT_URL_MAX {
        if open_url(&url) {
            return Outcome::Opened;
        }
        copy(publication.text.clone());
        return Outcome::OpenFailed;
    }
    copy(publication.text.clone());
    if open_url(&publication.new_file_url(false)) {
        Outcome::OpenedCopied
    }
    else {
        Outcome::OpenFailed
    }
}

/// Whether `text` holds something that looks local or private - the home
/// directory, an email address, a credential - by the bug report's scrub,
/// which removes exactly those before a log is published.
pub(super) fn has_local_values(text: &str, home: Option<&str>) -> bool {
    crate::bug_report::scrub(text, home) != text
}

#[cfg(test)]
mod tests;
