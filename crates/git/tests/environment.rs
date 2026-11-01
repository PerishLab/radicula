mod support;

use radicula_git::{CONTRACT, LOCAL, POLICY, detached};
use std::process::Command;
use support::{Repo, success};

#[test]
fn contract() {
    let contract: serde_json::Value = serde_json::from_str(CONTRACT).unwrap();
    assert_eq!(contract["policy"], POLICY);
    let keys = contract["environment"]["remove"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| key.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(keys, LOCAL);
    let command = detached("git");
    for &key in LOCAL {
        assert!(
            command
                .get_envs()
                .any(|(held, value)| held == key && value.is_none())
        );
    }
    for key in contract["environment"]["retain_examples"]
        .as_array()
        .unwrap()
    {
        let key = key.as_str().unwrap();
        assert!(!command.get_envs().any(|(held, _)| held == key));
    }
}

#[test]
fn inherited() {
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ambient", "--nocapture"])
        .env("RADICULA_GIT_CHILD", "1")
        .env("GIT_DIR", "absent-repository")
        .env("GIT_INDEX_FILE", "absent-index")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "probe.ambient")
        .env("GIT_CONFIG_VALUE_0", "injected")
        .env("GIT_AUTHOR_NAME", "Inherited Git")
        .env("GIT_AUTHOR_EMAIL", "inherited@example.invalid")
        .output()
        .unwrap();
    success(result);
}

#[test]
fn ambient() {
    if std::env::var_os("RADICULA_GIT_CHILD").is_none() {
        return;
    }
    let repo = Repo::new();
    assert_eq!(
        repo.run(&["config", "--get", "probe.ambient"])
            .status
            .code(),
        Some(1)
    );
    let author = success(repo.run(&["var", "GIT_AUTHOR_IDENT"]));
    assert!(author.starts_with(b"Inherited Git <inherited@example.invalid>"));
}

#[test]
fn explicit() {
    let repo = Repo::new();
    let output = repo
        .command(&["config", "--get", "probe.owned"])
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "probe.owned")
        .env("GIT_CONFIG_VALUE_0", "explicit")
        .output()
        .unwrap();
    assert_eq!(success(output), b"explicit\n");
}

#[test]
fn order() {
    let mut command = Command::new("git");
    command.env("GIT_INDEX_FILE", "before");
    radicula_git::detach(&mut command);
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "GIT_INDEX_FILE" && value.is_none())
    );
    command.env("GIT_INDEX_FILE", "after");
    assert!(command.get_envs().any(
        |(key, value)| key == "GIT_INDEX_FILE" && value == Some(std::ffi::OsStr::new("after"))
    ));
}
