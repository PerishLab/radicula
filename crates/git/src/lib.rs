mod context;
mod environment;
mod error;
mod output;
mod paths;
mod query;

pub use context::Git;
pub use environment::{LOCAL, POLICY, detach, detached};
pub use error::Error;
pub use output::Reply;
pub use paths::{Change, Status};
pub use query::Source;

pub const CONTRACT: &str = include_str!("../contract.json");
