mod support;

use std::io::Write;
use std::process::Stdio;
use support::{Repo, success};

#[test]
fn pen() {
    let repo = Repo::bare();
    let pen = repo.seat.path().join("pen");
    std::fs::create_dir(&pen).unwrap();
    let mut child = repo
        .command(&["hash-object", "-w", "--stdin"])
        .env("GIT_OBJECT_DIRECTORY", &pen)
        .env(
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            repo.git.root().join("objects"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"owned pen payload")
        .unwrap();
    let object = radicula_git::Reply::new(child.wait_with_output().unwrap())
        .object()
        .unwrap();
    assert!(pen.join(&object[..2]).join(&object[2..]).exists());
    assert!(
        !repo
            .git
            .root()
            .join("objects")
            .join(&object[..2])
            .join(&object[2..])
            .exists()
    );
    success(
        repo.command(&["cat-file", "-e", &object])
            .env("GIT_OBJECT_DIRECTORY", &pen)
            .env(
                "GIT_ALTERNATE_OBJECT_DIRECTORIES",
                repo.git.root().join("objects"),
            )
            .output()
            .unwrap(),
    );
    assert!(!repo.run(&["cat-file", "-e", &object]).status.success());
}

#[test]
fn index() {
    let repo = Repo::new();
    repo.commit();
    repo.file("owned", b"index overlay");
    let index = repo.seat.path().join("index");
    success(
        repo.command(&["read-tree", "HEAD"])
            .env("GIT_INDEX_FILE", &index)
            .output()
            .unwrap(),
    );
    success(
        repo.command(&["add", "owned"])
            .env("GIT_INDEX_FILE", &index)
            .output()
            .unwrap(),
    );
    let mut command = repo.git.changes(radicula_git::Source::Staged);
    repo.isolate(&mut command);
    command.env("GIT_INDEX_FILE", &index);
    assert_eq!(
        radicula_git::Reply::new(command.output().unwrap())
            .changes()
            .unwrap()
            .len(),
        1
    );
    assert!(
        radicula_git::Reply::new(
            repo.git
                .changes(radicula_git::Source::Staged)
                .output()
                .unwrap()
        )
        .changes()
        .unwrap()
        .is_empty()
    );
}
