mod system;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone)]
pub struct Search {
    pub(super) root: PathBuf,
    pub(super) path: Option<OsString>,
    pub(super) extensions: Option<OsString>,
}

impl Search {
    pub fn new(
        root: &Path,
        path: Option<OsString>,
        extensions: Option<OsString>,
    ) -> Result<Self, crate::Error> {
        let root = root.canonicalize().map_err(crate::Error::Io)?;
        if !root.is_dir() {
            return Err(crate::Error::Invalid("search root is not a directory"));
        }
        Ok(Self {
            root,
            path,
            extensions,
        })
    }

    pub fn resolve(&self, program: impl AsRef<OsStr>) -> Result<PathBuf, crate::Error> {
        let program = program.as_ref();
        let path = which::WhichConfig::new_with_sys(self)
            .custom_cwd(self.root.clone())
            .binary_name(program.to_os_string())
            .first_result()
            .map_err(crate::Error::Lookup)?;
        if cfg!(windows)
            && let Some(extension) = path.extension()
            && !["exe", "com", "cmd", "bat"]
                .iter()
                .any(|name| extension.eq_ignore_ascii_case(name))
        {
            return Err(crate::Error::Invalid("unsupported executable format"));
        }
        Ok(path)
    }

    pub fn command(&self, program: impl AsRef<OsStr>) -> Result<Command, crate::Error> {
        Ok(self.at(&self.resolve(program)?))
    }

    pub fn bind(&self, program: impl AsRef<OsStr>) -> Result<crate::Binding, crate::Error> {
        crate::Binding::new(self.clone(), program.as_ref())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn at(&self, path: &Path) -> Command {
        let mut command = Command::new(path);

        command.current_dir(&self.root);
        match &self.path {
            Some(path) => {
                command.env("PATH", path);
            }
            None => {
                command.env_remove("PATH");
            }
        }
        if cfg!(windows) {
            match &self.extensions {
                Some(extensions) => {
                    command.env("PATHEXT", extensions);
                }
                None => {
                    command.env_remove("PATHEXT");
                }
            }
        }
        command
    }
}
