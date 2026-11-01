#![allow(dead_code)]

use radicula_git::Git;
use std::path::Path;
use std::process::{Command, Output};

pub struct Repo {
    pub seat: tempfile::TempDir,
    pub git: Git,
}

impl Repo {
    pub fn new() -> Self {
        Self::make(false)
    }

    pub fn bare() -> Self {
        Self::make(true)
    }

    fn make(bare: bool) -> Self {
        let seat = tempfile::tempdir().unwrap();
        let root = seat.path().join("source");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(seat.path().join("configuration"), "").unwrap();
        let repo = Self {
            seat,
            git: Git::new("git", root),
        };
        let mut command = repo.command(&["init", "-q", "-b", "main"]);
        if bare {
            command.arg("--bare");
        }
        success(command.output().unwrap());
        repo
    }

    pub fn command(&self, args: &[&str]) -> Command {
        let mut command = self.git.command();
        self.isolate(&mut command);
        command.args(args);
        command
    }

    pub fn isolate(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.seat.path().join("configuration"));
    }

    pub fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    pub fn commit(&self) -> String {
        let output = self
            .command(&["commit", "--allow-empty", "-qm", "fixture"])
            .env("GIT_AUTHOR_NAME", "Git Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Git Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .output()
            .unwrap();
        success(output);
        radicula_git::Reply::new(self.git.head().output().unwrap())
            .object()
            .unwrap()
    }

    pub fn file(&self, path: impl AsRef<Path>, bytes: &[u8]) {
        std::fs::write(self.git.root().join(path), bytes).unwrap();
    }
}

pub fn success(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

pub fn output(code: i32, stdout: &[u8]) -> Output {
    Output {
        status: status(code),
        stdout: stdout.to_vec(),
        stderr: b"retained diagnostic".to_vec(),
    }
}

#[cfg(unix)]
fn status(code: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(code << 8)
}

#[cfg(windows)]
fn status(code: i32) -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(code as u32)
}
