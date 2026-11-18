use radicula_document::{Document, Error, Operation, Path, Presence};
use serde_json::{Value, json};

fn at(text: &str) -> Path {
    Path::parse(text).unwrap()
}

fn doc(text: &str) -> Document {
    Document::parse(text).unwrap()
}

#[test]
fn address() {
    assert_eq!(at("").parts().len(), 0);
    assert_eq!(at("/").parts(), [""]);
    assert_eq!(at("//").parts(), ["", ""]);
    assert_eq!(at("/a/").parts(), ["a", ""]);
    assert_eq!(at("/a~1b").parts(), ["a/b"]);
    assert_eq!(at("/a~0b").parts(), ["a~b"]);
    assert_eq!(Path::parse("a"), Err(Error::Pointer));
    assert_eq!(Path::parse("/a~"), Err(Error::Escape));
    assert_eq!(Path::parse("/a~2"), Err(Error::Escape));
}

#[test]
fn presence() {
    let held = doc(r#"{"a":null,"b":false,"c":0,"d":"","e":[]}"#);
    for key in ["a", "b", "c", "d", "e"] {
        let path = at(&format!("/{key}"));
        assert!(
            matches!(held.read(&path), Ok(Presence::Present(_))),
            "{key}"
        );
    }
    assert_eq!(held.read(&at("/gone")), Ok(Presence::Missing));
    assert_eq!(held.read(&at("/a/deep")), Err(Error::Kind));
    let present = held.read(&at("/b")).unwrap();
    assert_eq!(present.value(), Some(&json!(false)));
    assert_eq!(Presence::Missing.value(), None::<&Value>);
}

#[test]
fn write() {
    let mut held = doc(r#"{"a":1,"list":[10,20]}"#);
    held.set(&at("/a"), json!(null)).unwrap();
    held.set(&at("/b"), json!(2)).unwrap();
    held.set(&at("/list/0"), json!(30)).unwrap();
    assert_eq!(held.root(), &json!({"a": null, "b": 2, "list": [30, 20]}));
    held.set(&at(""), json!([1])).unwrap();
    assert_eq!(held.root(), &json!([1]));
}

#[test]
fn refuse() {
    let mut held = doc(r#"{"a":1,"list":[10]}"#);
    assert_eq!(held.set(&at("/gone/deep"), json!(2)), Err(Error::Missing));
    assert_eq!(held.set(&at("/list"), json!(2)), Ok(()));
    assert_eq!(held.set(&at("/list/0"), json!(3)), Err(Error::Kind));
    assert_eq!(held.remove(&at("")), Err(Error::Root));
    assert_eq!(held.remove(&at("/none")), Err(Error::Missing));
    assert_eq!(held.root(), &json!({"a": 1, "list": 2}));
}

#[test]
fn shift() {
    let mut held = doc(r#"{"list":["a","b","c"]}"#);
    held.remove(&at("/list/1")).unwrap();
    assert_eq!(held.root(), &json!({"list": ["a", "c"]}));
    assert_eq!(
        held.read(&at("/list/1")).unwrap().value(),
        Some(&json!("c"))
    );
    assert_eq!(held.read(&at("/list/2")), Ok(Presence::Missing));
}

#[test]
fn batch() {
    let mut held = doc(r#"{"a":1}"#);
    let steps = [
        Operation::Set {
            path: at("/b"),
            value: json!(2),
        },
        Operation::Remove { path: at("/c") },
    ];
    let fault = held.apply(&steps).unwrap_err();
    assert_eq!(fault.index, 1);
    assert_eq!(fault.error, Error::Missing);
    assert_eq!(held.root(), &json!({"a": 1}));
    held.apply(&[]).unwrap();
    assert_eq!(held.root(), &json!({"a": 1}));
}

#[test]
fn extent() {
    let mut held = doc("[1]");
    assert_eq!(held.read(&at("/1")), Ok(Presence::Missing));
    assert_eq!(
        held.read(&at("/18446744073709551616")),
        Ok(Presence::Missing)
    );
    assert_eq!(held.set(&at("/1"), json!(2)), Err(Error::Missing));
    assert_eq!(
        held.set(&at("/18446744073709551616"), json!(2)),
        Err(Error::Missing)
    );
    assert_eq!(
        held.remove(&at("/18446744073709551616")),
        Err(Error::Missing)
    );
    assert_eq!(held.read(&at("/01")), Err(Error::Index));
    let mut scalar = doc("1");
    assert_eq!(scalar.remove(&at("/a")), Err(Error::Kind));
    assert_eq!(scalar.set(&at("/a"), json!(1)), Err(Error::Kind));
    assert_eq!(scalar.root(), &json!(1));
}

#[test]
fn proto() {
    let mut held = doc("{}");
    held.set(&at("/__proto__"), json!({"a": 1})).unwrap();
    assert_eq!(held.text(), r#"{"__proto__":{"a":1}}"#);
    assert_eq!(
        held.read(&at("/__proto__/a")).unwrap().value(),
        Some(&json!(1))
    );
    assert_eq!(held.root(), &json!({"__proto__": {"a": 1}}));
    let made = doc(r#"{"__proto__":2}"#);
    assert_eq!(
        made.read(&at("/__proto__")).unwrap().value(),
        Some(&json!(2))
    );
}

#[test]
fn number() {
    let exact = "9007199254740991";
    let above = "9007199254740992";
    let below = "-9007199254740992";
    assert!(Document::parse(exact).is_ok());
    assert!(Document::parse(&format!("-{exact}")).is_ok());
    assert_eq!(Document::parse(above), Err(Error::Number));
    assert_eq!(Document::parse(below), Err(Error::Number));
    assert_eq!(
        Document::new(json!({"deep": [1, {"n": 9007199254740992u64}]})),
        Err(Error::Number)
    );
    assert_eq!(Document::parse("1.5"), Ok(doc("1.5")));
    assert_eq!(Document::parse("1.0"), Ok(doc("1.0")));
    assert_eq!(Document::parse("1e400"), Err(Error::Syntax));
    assert_eq!(Document::parse("1e20"), Err(Error::Number));
    assert_eq!(Document::parse("9007199254740992.0"), Err(Error::Number));
    assert_eq!(
        Document::parse("9007199254740991.0").unwrap().text(),
        "9007199254740991.0"
    );
    assert_eq!(Document::parse("-1e20"), Err(Error::Number));
    assert_eq!(Document::parse("1e-400").unwrap().text(), "0.0");
    assert_eq!(Document::parse("1E2").unwrap().text(), "100.0");
    let mut held = doc("{}");
    assert_eq!(
        held.set(&at("/n"), json!(9007199254740992u64)),
        Err(Error::Number)
    );
    assert_eq!(held.root(), &json!({}));
}

#[test]
fn unicode() {
    let mut held = doc(r#"{"名字":{"x.y":"值"}}"#);
    assert_eq!(
        held.read(&at("/名字/x.y")).unwrap().value(),
        Some(&json!("值"))
    );
    held.set(&at("/名字/a~0b"), json!("~")).unwrap();
    assert_eq!(
        held.read(&at("/名字/a~0b")).unwrap().value(),
        Some(&json!("~"))
    );
}

#[test]
fn text() {
    let held = doc(r#"{ "b" : 1, "a" : [ ] }"#);
    assert_eq!(held.text(), r#"{"a":[],"b":1}"#);
    assert_eq!(Document::parse("{"), Err(Error::Syntax));
    assert_eq!(Document::parse("nope"), Err(Error::Syntax));
    assert_eq!(Document::parse(""), Err(Error::Syntax));
    assert_eq!(held.take(), json!({"a": [], "b": 1}));
}
