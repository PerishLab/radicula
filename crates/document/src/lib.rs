#![forbid(unsafe_code)]

mod document;
mod edit;
mod pointer;

pub use document::Document;
pub use pointer::Path;
pub use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Escape,
    Index,
    Kind,
    Missing,
    Number,
    Pointer,
    Root,
    Syntax,
}

impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Escape => "escape",
            Self::Index => "index",
            Self::Kind => "kind",
            Self::Missing => "missing",
            Self::Number => "number",
            Self::Pointer => "pointer",
            Self::Root => "root",
            Self::Syntax => "syntax",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, form: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Escape => "invalid ~ escape in a path segment",
            Self::Index => "array path segment is not a canonical index",
            Self::Kind => "path traverses a value of another kind",
            Self::Missing => "path location is absent",
            Self::Number => "integer exceeds the exact JSON domain",
            Self::Pointer => "path must be empty or start with /",
            Self::Root => "the document root cannot be removed",
            Self::Syntax => "text is not valid JSON",
        };
        form.write_str(text)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Presence<'a> {
    Missing,
    Present(&'a Value),
}

impl<'a> Presence<'a> {
    pub fn value(self) -> Option<&'a Value> {
        match self {
            Self::Missing => None,
            Self::Present(held) => Some(held),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Set { path: Path, value: Value },
    Remove { path: Path },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub index: usize,
    pub error: Error,
}
