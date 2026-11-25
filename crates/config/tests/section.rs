use radicula_config::Error;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Inspect {
    depth: u32,
    label: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Locus {
    inspect: Inspect,
}

fn store(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("fixture should be written");
    path
}

#[test]
fn absent() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("absent.toml");
    let held = radicula_config::section::<Inspect>(&path, "inspect").unwrap();
    assert!(held.is_none());
}

#[test]
fn vacant() {
    let root = tempfile::tempdir().unwrap();
    let path = store(root.path(), "held.toml", "name = \"held\"\n");
    let held = radicula_config::section::<Inspect>(&path, "inspect").unwrap();
    assert!(held.is_none());
}

#[test]
fn hollow() {
    let root = tempfile::tempdir().unwrap();
    let path = store(root.path(), "held.toml", "[locus]\nname = \"held\"\n");
    let held = radicula_config::section::<Inspect>(&path, "locus.inspect").unwrap();
    assert!(held.is_none());
    let held = radicula_config::section::<Inspect>(&path, "other.inspect").unwrap();
    assert!(held.is_none());
}

#[cfg(unix)]
#[test]
fn sealed() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let path = store(root.path(), "held.toml", "[inspect]\ndepth = 1\n");
    let mut mode = std::fs::metadata(&path).unwrap().permissions();
    mode.set_mode(0o000);
    std::fs::set_permissions(&path, mode).unwrap();
    let readable = std::fs::read_to_string(&path).is_ok();
    if readable {
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
        return;
    }
    let error = radicula_config::section::<Inspect>(&path, "inspect").unwrap_err();
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
    match error {
        Error::Read { path: held, .. } => assert_eq!(held, path),
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn folder() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("held");
    std::fs::create_dir(&path).unwrap();
    let error = radicula_config::section::<Inspect>(&path, "inspect").unwrap_err();
    match error {
        Error::Read { path: held, .. } => assert_eq!(held, path),
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn syntax() {
    let root = tempfile::tempdir().unwrap();
    let path = store(root.path(), "held.toml", "inspect =\n");
    let error = radicula_config::section::<Inspect>(&path, "inspect").unwrap_err();
    assert!(error.to_string().contains(&path.display().to_string()));
    match error {
        Error::Parse { path: held, .. } => assert_eq!(held, path),
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn scalar() {
    let root = tempfile::tempdir().unwrap();
    let path = store(root.path(), "held.toml", "inspect = 7\n");
    let error = radicula_config::section::<Inspect>(&path, "inspect").unwrap_err();
    match error {
        Error::Section {
            path: held,
            section,
            why,
        } => {
            assert_eq!(held, path);
            assert_eq!(section, "inspect");
            assert!(why.contains("not a table"), "{why}");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn inner() {
    let root = tempfile::tempdir().unwrap();
    let path = store(root.path(), "held.toml", "[locus]\ninspect = 7\n");
    let error = radicula_config::section::<Inspect>(&path, "locus.inspect").unwrap_err();
    match error {
        Error::Section {
            path: held,
            section,
            why,
        } => {
            assert_eq!(held, path);
            assert_eq!(section, "locus.inspect");
            assert!(why.contains("not a table"), "{why}");
            assert!(why.contains("`inspect`"), "{why}");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn dotted() {
    let root = tempfile::tempdir().unwrap();
    let path = store(
        root.path(),
        "held.toml",
        "[locus.inspect]\ndepth = 2\nlabel = \"fine\"\n",
    );
    let held = radicula_config::section::<Inspect>(&path, "locus.inspect")
        .unwrap()
        .expect("section should be present");
    assert_eq!(
        held,
        Inspect {
            depth: 2,
            label: Some("fine".into()),
        }
    );
}

#[test]
fn nested() {
    let root = tempfile::tempdir().unwrap();
    let path = store(
        root.path(),
        "held.toml",
        "[locus]\n[locus.inspect]\ndepth = 3\n",
    );
    let held = radicula_config::section::<Locus>(&path, "locus")
        .unwrap()
        .expect("section should be present");
    assert_eq!(
        held,
        Locus {
            inspect: Inspect {
                depth: 3,
                label: None,
            },
        }
    );
}

#[test]
fn unknown() {
    let root = tempfile::tempdir().unwrap();
    let path = store(
        root.path(),
        "held.toml",
        "[locus.inspect]\ndepth = 1\nextra = true\n",
    );
    let error = radicula_config::section::<Inspect>(&path, "locus.inspect").unwrap_err();
    match error {
        Error::Section {
            path: held,
            section,
            why,
        } => {
            assert_eq!(held, path);
            assert_eq!(section, "locus.inspect");
            assert!(why.contains("unknown field"), "{why}");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn refused() {
    let root = tempfile::tempdir().unwrap();
    let path = store(root.path(), "held.toml", "[inspect]\ndepth = \"bad\"\n");
    let error = radicula_config::section::<Inspect>(&path, "inspect").unwrap_err();
    match error {
        Error::Section {
            path: held,
            section,
            why,
        } => {
            assert_eq!(held, path);
            assert_eq!(section, "inspect");
            assert!(
                why.contains("invalid type") || why.contains("invalid"),
                "{why}"
            );
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn empty() {
    let root = tempfile::tempdir().unwrap();
    let path = store(root.path(), "held.toml", "[inspect]\ndepth = 1\n");
    let error = radicula_config::section::<Inspect>(&path, "").unwrap_err();
    match error {
        Error::Section { section, why, .. } => {
            assert!(section.is_empty());
            assert!(why.contains("empty"), "{why}");
        }
        other => panic!("unexpected error: {other}"),
    }
    let error = radicula_config::section::<Inspect>(&path, "locus.").unwrap_err();
    match error {
        Error::Section { section, why, .. } => {
            assert_eq!(section, "locus.");
            assert!(why.contains("empty segment"), "{why}");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn quoted() {
    let root = tempfile::tempdir().unwrap();
    let path = store(
        root.path(),
        "held.toml",
        "\"locus.inspect\" = { depth = 4 }\n",
    );
    let held = radicula_config::section::<Inspect>(&path, "locus.inspect").unwrap();
    assert!(held.is_none());
}
