use super::Search;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use which::sys::RealSys;

impl which::sys::Sys for &Search {
    type ReadDirEntry = std::fs::DirEntry;
    type Metadata = std::fs::Metadata;

    fn is_windows(&self) -> bool {
        cfg!(windows)
    }
    fn current_dir(&self) -> io::Result<PathBuf> {
        Ok(self.root.clone())
    }
    fn home_dir(&self) -> Option<PathBuf> {
        None
    }
    fn env_split_paths(&self, paths: &OsStr) -> Vec<PathBuf> {
        RealSys.env_split_paths(paths)
    }
    fn env_path(&self) -> Option<OsString> {
        self.path.clone()
    }
    fn env_path_ext(&self) -> Option<OsString> {
        self.extensions.clone()
    }
    fn metadata(&self, path: &Path) -> io::Result<Self::Metadata> {
        RealSys.metadata(path)
    }
    fn symlink_metadata(&self, path: &Path) -> io::Result<Self::Metadata> {
        RealSys.symlink_metadata(path)
    }
    fn read_dir(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<Self::ReadDirEntry>>>> {
        RealSys.read_dir(path)
    }
    fn is_valid_executable(&self, path: &Path) -> io::Result<bool> {
        RealSys.is_valid_executable(path)
    }
}
