use radicula_template::{Error, fill, marked};
use std::collections::BTreeMap;

#[test]
fn literal() {
    let vars = BTreeMap::from([
        ("nested", "{absent}{@absent}".to_string()),
        ("empty", String::new()),
    ]);
    assert_eq!(
        fill("{nested}:{empty}", &vars).unwrap(),
        "{absent}{@absent}:"
    );
    assert_eq!(
        marked("{@nested}:{@empty}", &vars).unwrap(),
        "{absent}{@absent}:"
    );
}

#[test]
fn unicode() {
    let vars = BTreeMap::from([("名字", "世界".to_string())]);
    assert_eq!(fill("你好 {名字} {名字}", &vars).unwrap(), "你好 世界 世界");
    assert_eq!(
        marked("你好 {@名字} {普通} }", &vars).unwrap(),
        "你好 世界 {普通} }"
    );
}

#[test]
fn names() {
    let vars = BTreeMap::from([(" name ", "exact".to_string())]);
    assert_eq!(fill("{ name }", &vars).unwrap(), "exact");
    assert_eq!(marked("{@ name }", &vars).unwrap(), "exact");
    assert_eq!(marked("{@}", &vars), Err(Error::Empty));
    assert_eq!(marked("{ } } {{", &vars).unwrap(), "{ } } {{");
}
