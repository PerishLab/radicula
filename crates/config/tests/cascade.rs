use radicula_config::Cascade;
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Debug, Deserialize, PartialEq, radicula_config::Cascade)]
#[cascade(section, strict)]
#[serde(default)]
struct Bind {
    host: String,
    port: u16,
}

impl Default for Bind {
    fn default() -> Self {
        Bind {
            host: "127.0.0.1".to_string(),
            port: 3000,
        }
    }
}

#[derive(Debug, PartialEq, radicula_config::Cascade)]
#[cascade(strict)]
struct Rig {
    name: String,
    #[cascade(name = "legacy_name")]
    alias: String,
    count: u16,
    #[cascade(section)]
    listen: Bind,
    #[cascade(arg)]
    theme: String,
    #[cascade(arg)]
    tag: Option<String>,
    extras: BTreeMap<String, String>,
}

impl Default for Rig {
    fn default() -> Self {
        Rig {
            name: "base".to_string(),
            alias: "plain".to_string(),
            count: 4,
            listen: Bind::default(),
            theme: "plain".to_string(),
            tag: None,
            extras: BTreeMap::new(),
        }
    }
}

fn nothing(_: &str) -> Option<String> {
    None
}

#[test]
fn defaults() {
    let held = Rig::default().merge(Rig::lookup("RIG", &nothing).expect("env should read"));
    assert_eq!(held, Rig::default());
}

#[test]
fn filed() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let dir = scratch.path().join("plumb-cascade-file");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    let path = dir.join("rig.toml");
    std::fs::write(
        &path,
        "name = \"filed\"\nlegacy_name = \"carried\"\n[listen]\nport = 9\n[extras]\nkey = \"value\"\n",
    )
    .expect("file should be written");
    let over = radicula_config::load::<RigPartial>(&path).expect("file should parse");
    let held = Rig::default().merge(over);
    assert_eq!(held.name, "filed");
    assert_eq!(held.alias, "carried");
    assert_eq!(held.listen.port, 9);
    assert_eq!(held.listen.host, "127.0.0.1");
    assert_eq!(held.count, 4);
    assert_eq!(held.extras.get("key").map(String::as_str), Some("value"));
}

#[test]
fn strict() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let dir = scratch.path().join("plumb-cascade-strict");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    let path = dir.join("rig.toml");

    std::fs::write(&path, "unknown = true\n").expect("file should be written");
    let root =
        Rig::resolve(Some(&path), "RIG", &nothing).expect_err("unknown root field should fail");
    assert!(root.to_string().contains("unknown field"), "{root}");

    std::fs::write(&path, "[listen]\nunknown = true\n").expect("file should be written");
    let section =
        Rig::resolve(Some(&path), "RIG", &nothing).expect_err("unknown section field should fail");
    assert!(section.to_string().contains("unknown field"), "{section}");
}

#[test]
fn veiled() {
    let get = |key: &str| match key {
        "RIG_NAME" => Some("veiled".to_string()),
        "RIG_LEGACY_NAME" => Some("mapped".to_string()),
        "RIG_LISTEN_PORT" => Some("7".to_string()),
        "RIG_TAG" => Some("held".to_string()),
        _ => None,
    };
    let held = Rig {
        name: "filed".to_string(),
        ..Rig::default()
    }
    .merge(Rig::lookup("RIG", &get).expect("env should read"));
    assert_eq!(held.name, "veiled");
    assert_eq!(held.alias, "mapped");
    assert_eq!(held.listen.port, 7);
    assert_eq!(held.tag.as_deref(), Some("held"));
}

#[test]
fn armed() {
    let get = |key: &str| (key == "RIG_THEME").then(|| "enved".to_string());
    let args = RigArgs {
        theme: Some("dark".to_string()),
        tag: Some("tipped".to_string()),
    };
    let held = Rig::default()
        .merge(Rig::lookup("RIG", &get).expect("env should read"))
        .merge(args.partial());
    assert_eq!(held.theme, "dark");
    assert_eq!(held.tag.as_deref(), Some("tipped"));
}

#[test]
fn optional() {
    #[derive(clap::Parser)]
    struct Cli {
        #[command(flatten)]
        over: RigArgs,
    }
    let bare = <Cli as clap::Parser>::try_parse_from(["rig"]).expect("bare parse should hold");
    assert_eq!(bare.over.theme, None);
    assert_eq!(bare.over.tag, None);
    let full = <Cli as clap::Parser>::try_parse_from(["rig", "--theme", "dark", "--tag", "t"])
        .expect("full parse should hold");
    assert_eq!(full.over.theme.as_deref(), Some("dark"));
    assert_eq!(full.over.tag.as_deref(), Some("t"));
}

#[test]
fn refused() {
    let get = |key: &str| (key == "RIG_COUNT").then(|| "nope".to_string());
    let err = Rig::lookup("RIG", &get).expect_err("the parse should fail");
    assert!(err.to_string().contains("RIG_COUNT"), "{err}");
}

#[test]
fn blank() {
    let get = |key: &str| (key == "RIG_NAME").then(|| "  ".to_string());
    let held = Rig::default().merge(Rig::lookup("RIG", &get).expect("env should read"));
    assert_eq!(held.name, "base");
}

#[test]
fn blind() {
    let get = |key: &str| (key == "RIG_EXTRAS").then(|| "poked".to_string());
    let held = Rig::default().merge(Rig::lookup("RIG", &get).expect("env should read"));
    assert!(held.extras.is_empty());
}

#[test]
fn onion() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let dir = scratch.path().join("plumb-cascade-resolve");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    let path = dir.join("rig.toml");
    std::fs::write(&path, "name = \"filed\"\ncount = 2\n").expect("file should be written");
    let held = Rig::resolve(Some(&path), "RIG", &nothing).expect("the onion should resolve");
    assert_eq!(held.name, "filed");
    assert_eq!(held.count, 2);
    assert_eq!(held.theme, "plain");
    let over = RigPartial {
        count: Some(9),
        ..RigPartial::default()
    };
    let held =
        Rig::layered(Some(&path), "RIG", &nothing, over).expect("the top layer should resolve");
    assert_eq!(held.name, "filed");
    assert_eq!(held.count, 9);
}
