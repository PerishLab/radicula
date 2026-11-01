use radicula_process::{Environment, Search, Tool};
use std::ffi::{OsStr, OsString};
use std::process::Command;

#[test]
fn fingerprint() {
    let seat = tempfile::tempdir().unwrap();
    let path = seat.path().join("data");
    std::fs::write(&path, b"probe\n").unwrap();
    let tool = Tool::at(path.clone()).unwrap();
    assert_eq!(tool.path, path);
    assert_eq!(
        tool.digest,
        "25be323556dad377abb57fe7ec8c4b99a6527f488dda28d0c9b686528659c909"
    );
    tool.verify().unwrap();
    let encoded = serde_json::to_value(&tool).unwrap();
    assert_eq!(encoded.as_object().unwrap().len(), 2);
    assert_eq!(encoded["digest"], tool.digest);
    assert_eq!(
        serde_json::from_value::<Tool>(encoded.clone()).unwrap(),
        tool
    );
    let mut extra = encoded;
    extra["unrecognized"] = true.into();
    assert!(serde_json::from_value::<Tool>(extra).is_err());
    std::fs::write(&path, b"changed").unwrap();
    assert!(tool.verify().is_err());
    assert!(Tool::at(seat.path().join("absent")).is_err());
}

#[test]
fn environment() {
    let env = Environment::new([
        (OsString::from("PUBLIC"), OsString::from("selected")),
        (OsString::from("REMOVE"), OsString::from("ambient")),
    ]);
    let mut command = Command::new("program");
    env.apply(&mut command);
    command.env("OWNED", "explicit").env_remove("REMOVE");
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "PUBLIC" && value == Some(OsStr::new("selected")))
    );
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "OWNED" && value == Some(OsStr::new("explicit")))
    );
    assert!(
        !command
            .get_envs()
            .any(|(key, value)| key == "REMOVE" && value.is_some())
    );
    assert!(!format!("{env:?}").contains("selected"));
    assert_eq!(env.values().len(), 2);
}

#[test]
fn search() {
    let root = tempfile::tempdir().unwrap();
    let search = Search::new(
        root.path(),
        Some(OsString::new()),
        Some(OsString::from(".EXE")),
    )
    .unwrap();
    assert!(search.resolve("absent-owned-program").is_err());
    let exe = std::env::current_exe().unwrap();
    let binding = search.bind(exe.as_os_str()).unwrap();
    assert_eq!(binding.tool().path, exe);
    binding.verify().unwrap();
    let command = binding.command().unwrap();
    assert_eq!(command.get_program(), exe);
    assert_eq!(
        command.get_current_dir(),
        Some(root.path().canonicalize().unwrap().as_path())
    );
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "PATH" && value == Some(OsStr::new("")))
    );
    assert!(Search::new(&root.path().join("absent"), None, None).is_err());
}

#[cfg(unix)]
#[test]
fn alias() {
    use std::os::unix::ffi::OsStringExt;
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("contents");
    std::fs::write(&target, b"hash identity").unwrap();
    let alias = root.path().join("launch");
    std::os::unix::fs::symlink(&target, &alias).unwrap();
    let tool = Tool::at(alias.clone()).unwrap();
    assert_eq!(tool.path, alias);
    assert_eq!(tool.content().unwrap(), target);
    let raw = root.path().join(OsString::from_vec(b"raw-\xff".to_vec()));
    std::fs::write(&raw, b"bytes").unwrap();
    let tool = Tool::at(raw.clone()).unwrap();
    assert_eq!(tool.path, raw);
    tool.verify().unwrap();
    assert!(serde_json::to_value(&tool).is_err());
}

#[cfg(windows)]
#[test]
fn extensions() {
    let root = tempfile::tempdir().unwrap();
    let exe = std::env::current_exe().unwrap();
    let search = Search::new(
        root.path(),
        exe.parent().map(|p| p.as_os_str().to_owned()),
        Some(".EXE".into()),
    )
    .unwrap();
    let name = exe.file_stem().unwrap();
    assert_eq!(search.resolve(name).unwrap(), exe);
    let absent = Search::new(
        root.path(),
        exe.parent().map(|p| p.as_os_str().to_owned()),
        Some(".COM".into()),
    )
    .unwrap();
    assert!(absent.resolve(name).is_err());
}

#[test]
fn shadowed() {
    let seat = tempfile::tempdir().unwrap();
    let first = seat.path().join("first");
    std::fs::create_dir(&first).unwrap();
    let exe = std::env::current_exe().unwrap();
    let second = exe.parent().unwrap();
    let paths = std::env::join_paths([first.as_path(), second]).unwrap();
    let search = Search::new(seat.path(), Some(paths), Some(".EXE".into())).unwrap();
    let binding = search.bind(exe.file_name().unwrap()).unwrap();
    binding.verify().unwrap();
    std::fs::copy(&exe, first.join(exe.file_name().unwrap())).unwrap();
    assert!(binding.verify().is_err());
    assert!(binding.command().is_err());
}
