use crate::Git;
use std::ffi::{OsStr, OsString};
use std::process::Command;

#[derive(Clone, Copy, Debug)]
pub enum Source<'a> {
    Staged,
    Worktree,
    Range(&'a OsStr),
}

impl Git {
    pub fn common(&self) -> Command {
        self.arguments(&["rev-parse", "--path-format=absolute", "--git-common-dir"])
    }

    pub fn directory(&self) -> Command {
        self.arguments(&["rev-parse", "--path-format=absolute", "--git-dir"])
    }

    pub fn bare(&self) -> Command {
        self.arguments(&["rev-parse", "--is-bare-repository"])
    }

    pub fn head(&self) -> Command {
        self.arguments(&["rev-parse", "--verify", "--end-of-options", "HEAD^{commit}"])
    }

    pub fn tree(&self, revision: impl AsRef<OsStr>) -> Command {
        let mut revision = OsString::from(revision.as_ref());
        revision.push("^{tree}");
        self.arguments(&[
            OsStr::new("rev-parse"),
            OsStr::new("--verify"),
            OsStr::new("--end-of-options"),
            &revision,
        ])
    }

    pub fn ancestor(&self, left: impl AsRef<OsStr>, right: impl AsRef<OsStr>) -> Command {
        self.arguments(&[
            OsStr::new("merge-base"),
            OsStr::new("--is-ancestor"),
            OsStr::new("--"),
            left.as_ref(),
            right.as_ref(),
        ])
    }

    pub fn changes(&self, source: Source<'_>) -> Command {
        let mut command = self.arguments(&[
            "diff",
            "--name-status",
            "-z",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--find-renames",
        ]);
        match source {
            Source::Staged => {
                command.arg("--cached");
            }
            Source::Worktree => {}
            Source::Range(range) => {
                command.args([OsStr::new("--end-of-options"), range]);
            }
        }
        command.arg("--");
        command
    }
}
