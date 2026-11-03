use radicula_config::{Cascade, Env, Error};
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq)]
struct Choice(String);

impl Env for Choice {
    fn read(value: &str) -> Result<Self, String> {
        value
            .strip_prefix("choice:")
            .map(|value| Self(value.into()))
            .ok_or_else(|| "expected choice:".into())
    }
}

impl<'de> serde::Deserialize<'de> for Choice {
    fn deserialize<D: serde::Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        String::deserialize(input).map(Self)
    }
}

#[derive(Debug, Default, Cascade)]
struct Settings {
    enabled: bool,
    path: PathBuf,
    label: Option<String>,
    choice: Choice,
}

#[test]
fn explicit() {
    let seen = std::cell::RefCell::new(Vec::new());
    let values = std::collections::BTreeMap::from([
        ("EXACT_ENABLED", "true"),
        ("EXACT_PATH", " ./data "),
        ("EXACT_LABEL", " label "),
        ("EXACT_CHOICE", " choice:one "),
    ]);
    let get = |key: &str| {
        seen.borrow_mut().push(key.to_string());
        values.get(key).map(|value| value.to_string())
    };
    let held = Settings::resolve(None, "EXACT", &get).unwrap();
    assert!(held.enabled);
    assert_eq!(held.path, PathBuf::from("./data"));
    assert_eq!(held.label.as_deref(), Some("label"));
    assert_eq!(held.choice, Choice("one".into()));
    assert_eq!(
        *seen.borrow(),
        ["EXACT_ENABLED", "EXACT_PATH", "EXACT_LABEL", "EXACT_CHOICE"]
    );
}

#[test]
fn cleared() {
    let held = Settings {
        label: Some("held".into()),
        ..Default::default()
    };
    let unchanged = held.merge(SettingsPartial::default());
    assert_eq!(unchanged.label.as_deref(), Some("held"));
    let cleared = unchanged.merge(SettingsPartial {
        label: Some(None),
        ..Default::default()
    });
    assert!(cleared.label.is_none());
}

#[test]
fn refused() {
    let get = |key: &str| (key == "EXACT_ENABLED").then(|| "TRUE".into());
    match Settings::lookup("EXACT", &get).unwrap_err() {
        Error::Env { key, why } => {
            assert_eq!(key, "EXACT_ENABLED");
            assert_eq!(why, "neither true nor false");
        }
        error => panic!("unexpected error: {error}"),
    }
}

#[test]
fn failures() {
    use std::error::Error as _;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.toml");
    let error = Settings::resolve(Some(&path), "EXACT", &|_| None).unwrap_err();
    assert!(matches!(error, Error::Read { .. }));
    assert!(error.source().is_some());
    assert!(error.to_string().contains(&path.display().to_string()));
    std::fs::write(&path, "enabled = 'bad'\n").unwrap();
    let error = Settings::resolve(Some(&path), "EXACT", &|_| None).unwrap_err();
    assert!(matches!(error, Error::Parse { .. }));
    assert!(error.source().is_some());
}
