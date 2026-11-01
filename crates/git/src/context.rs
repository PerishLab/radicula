use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Git {
    program: PathBuf,
    root: PathBuf,
}

impl Git {
    pub fn new(program: impl Into<PathBuf>, root: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            root: root.into(),
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn command(&self) -> Command {
        let mut command = crate::detached(&self.program);
        command.current_dir(&self.root);
        command
    }

    pub(crate) fn arguments(&self, args: &[impl AsRef<OsStr>]) -> Command {
        let mut command = self.command();
        command.args(args);
        command
    }
}
