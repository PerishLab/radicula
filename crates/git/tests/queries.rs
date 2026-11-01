mod support;

use radicula_git::{Git, Source, Status};
use support::{Repo, success};

#[test]
fn identity() {
    let repo = Repo::new();
    repo.commit();
    let common = radicula_git::Reply::new(repo.git.common().output().unwrap())
        .path()
        .unwrap();
    let directory = radicula_git::Reply::new(repo.git.directory().output().unwrap())
        .path()
        .unwrap();
    assert_eq!(
        common.canonicalize().unwrap(),
        directory.canonicalize().unwrap()
    );
    assert!(
        !radicula_git::Reply::new(repo.git.bare().output().unwrap())
            .flag()
            .unwrap()
    );
    let member = repo.seat.path().join("member");
    success(
        repo.command(&["worktree", "add", "-q", "-b", "member"])
            .arg(&member)
            .output()
            .unwrap(),
    );
    let linked = Git::new("git", member);
    let other = radicula_git::Reply::new(linked.common().output().unwrap())
        .path()
        .unwrap();
    assert_eq!(
        common.canonicalize().unwrap(),
        other.canonicalize().unwrap()
    );
    assert_ne!(
        radicula_git::Reply::new(linked.directory().output().unwrap())
            .path()
            .unwrap(),
        directory
    );
}

#[test]
fn bare() {
    let repo = Repo::bare();
    assert!(
        radicula_git::Reply::new(repo.git.bare().output().unwrap())
            .flag()
            .unwrap()
    );
    assert_eq!(
        radicula_git::Reply::new(repo.git.common().output().unwrap())
            .path()
            .unwrap()
            .canonicalize()
            .unwrap(),
        repo.git.root().canonicalize().unwrap()
    );
    assert!(
        radicula_git::Reply::new(repo.git.head().output().unwrap())
            .object()
            .is_err()
    );
}

#[test]
fn revisions() {
    let repo = Repo::new();
    let first = repo.commit();
    let second = repo.commit();
    assert_ne!(first, second);
    assert_eq!(
        radicula_git::Reply::new(repo.git.tree("HEAD").output().unwrap())
            .object()
            .unwrap()
            .len(),
        40
    );
    assert!(
        radicula_git::Reply::new(repo.git.ancestor(&first, &second).output().unwrap())
            .ancestry()
            .unwrap()
    );
    assert!(
        !radicula_git::Reply::new(repo.git.ancestor(&second, &first).output().unwrap())
            .ancestry()
            .unwrap()
    );
    assert!(
        radicula_git::Reply::new(
            repo.git
                .ancestor("not-a-revision", &second)
                .output()
                .unwrap()
        )
        .ancestry()
        .is_err()
    );
    assert!(
        radicula_git::Reply::new(repo.git.tree("--help").output().unwrap())
            .object()
            .is_err()
    );
}

#[test]
fn differences() {
    let repo = Repo::new();
    let first = repo.commit();
    repo.file("a space and unicode-文", b"first\n");
    success(repo.run(&["add", "--all"]));
    let staged = radicula_git::Reply::new(repo.git.changes(Source::Staged).output().unwrap())
        .changes()
        .unwrap();
    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].status, Status::Added);
    assert_eq!(
        staged[0].path,
        std::path::Path::new("a space and unicode-文")
    );
    repo.commit();
    let range = radicula_git::Reply::new(
        repo.git
            .changes(Source::Range(first.as_ref()))
            .output()
            .unwrap(),
    )
    .changes()
    .unwrap();
    assert_eq!(range.len(), 1);
    repo.file("a space and unicode-文", b"changed\n");
    let worktree = radicula_git::Reply::new(repo.git.changes(Source::Worktree).output().unwrap())
        .changes()
        .unwrap();
    assert_eq!(worktree[0].status, Status::Modified);
}

#[test]
fn renamed() {
    let repo = Repo::new();
    repo.file("original", b"preserve exact content\n");
    success(repo.run(&["add", "--all"]));
    repo.commit();
    success(repo.run(&["mv", "original", "renamed"]));
    let found = radicula_git::Reply::new(repo.git.changes(Source::Staged).output().unwrap())
        .changes()
        .unwrap();
    assert_eq!(found[0].status, Status::Renamed);
    assert_eq!(found[0].score, Some(100));
    assert_eq!(
        found[0].previous.as_deref(),
        Some(std::path::Path::new("original"))
    );
    assert_eq!(found[0].path, std::path::Path::new("renamed"));
}

#[cfg(unix)]
#[test]
fn unusual() {
    changed(std::ffi::OsStr::new("tab\tline\nunicode-文"));
}

#[cfg(target_os = "linux")]
#[test]
fn native() {
    use std::os::unix::ffi::OsStrExt;
    changed(std::ffi::OsStr::from_bytes(b"tab\tline\nraw-\xff"));
}

#[cfg(unix)]
fn changed(name: &std::ffi::OsStr) {
    let repo = Repo::new();
    repo.commit();
    repo.file(name, b"bytes");
    success(repo.run(&["add", "--all"]));
    let found = radicula_git::Reply::new(repo.git.changes(Source::Staged).output().unwrap())
        .changes()
        .unwrap();
    assert_eq!(found[0].path.as_os_str(), name);
}

#[tokio::test(flavor = "current_thread")]
async fn asynchronous() {
    let repo = Repo::new();
    let expected = repo.commit();
    let output = tokio::process::Command::from(repo.git.head())
        .output()
        .await
        .unwrap();
    assert_eq!(radicula_git::Reply::new(output).object().unwrap(), expected);
}

#[test]
fn launch() {
    let git = Git::new("a deliberate launch alias", "relative root");
    assert_eq!(git.command().get_program(), "a deliberate launch alias");
    assert_eq!(
        git.command().get_current_dir(),
        Some(std::path::Path::new("relative root"))
    );
}
