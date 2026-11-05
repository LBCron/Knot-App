use thiserror::Error;

pub type Result<T, E = LibraryError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("{0} is not owner/repo")]
    InvalidGitHubRepo(String),
    #[error("{0} is not an https:// address")]
    InvalidWebBase(String),
    #[error("{0} is not a folder")]
    InvalidFolder(String),
    #[error("{0} escapes the location")]
    PathEscapesLocation(String),
    #[error("index format {0} is not supported, this Knot understands format 1")]
    UnsupportedFormat(u32),
    #[error("item body is empty after trimming front matter")]
    EmptyBody,
    #[error("a title is one line of 1 to {0} characters")]
    InvalidTitle(usize),
    #[error("a description is one line of 1 to {0} characters")]
    InvalidDescription(usize),
    #[error("tag {0} is not 1 to 32 of a-z, 0-9 and -")]
    InvalidTag(String),
    #[error("an item carries at most {0} tags")]
    TooManyTags(usize),
    #[error("item body has no closing `---` for its front matter")]
    UnterminatedFrontMatter,
    #[error("item size does not match the index: expected {expected}, got {actual}")]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("item sha256 does not match the index: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("couldn't reach the location: {0}")]
    Fetch(String),
    #[error("couldn't parse the index: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("{0}")]
    Io(String),
}
