use std::fs;

use radicula_document::{Document, Error, Operation, Path, Presence};
use serde_json::{Value, json};

fn data() -> Value {
    let text = fs::read_to_string("spec/conformance.json").unwrap();
    serde_json::from_str(&text).unwrap()
}

fn cases() -> Vec<Value> {
    data()["cases"].as_array().unwrap().clone()
}

fn codes() -> Vec<Value> {
    data()["errors"].as_array().unwrap().clone()
}

fn fault(error: Error) -> Value {
    json!({"error": error.code()})
}

fn run(case: &Value) -> Value {
    let text = case["document"].as_str().unwrap();
    let mut held = match Document::parse(text) {
        Ok(held) => held,
        Err(error) => return fault(error),
    };
    let op = &case["op"];
    if op.get("text").is_some() {
        return json!({"text": held.text()});
    }
    if let Some(text) = op.get("read").and_then(Value::as_str) {
        return read(&held, text);
    }
    if let Some(step) = op.get("set") {
        let path = step["path"].as_str().unwrap();
        let result = locate(path).and_then(|path| held.set(&path, step["value"].clone()));
        return edited(&held, result);
    }
    if let Some(text) = op.get("remove").and_then(Value::as_str) {
        let result = locate(text).and_then(|path| held.remove(&path));
        return edited(&held, result);
    }
    let steps = op["apply"].as_array().unwrap();
    let batch: Vec<Operation> = steps.iter().map(step).collect();
    match held.apply(&batch) {
        Ok(()) => json!({"document": held.root()}),
        Err(fault) => json!({
            "error": fault.error.code(),
            "index": fault.index,
            "document": held.root()
        }),
    }
}

fn read(held: &Document, text: &str) -> Value {
    let path = match Path::parse(text) {
        Ok(path) => path,
        Err(error) => return fault(error),
    };
    match held.read(&path) {
        Ok(Presence::Present(value)) => json!({"presence": "present", "value": value}),
        Ok(Presence::Missing) => json!({"presence": "missing"}),
        Err(error) => fault(error),
    }
}

fn locate(text: &str) -> Result<Path, Error> {
    Path::parse(text)
}

fn edited(held: &Document, result: Result<(), Error>) -> Value {
    match result {
        Ok(()) => json!({"document": held.root()}),
        Err(error) => json!({"error": error.code(), "document": held.root()}),
    }
}

fn step(step: &Value) -> Operation {
    if let Some(step) = step.get("set") {
        return Operation::Set {
            path: locate(step["path"].as_str().unwrap()).unwrap(),
            value: step["value"].clone(),
        };
    }
    Operation::Remove {
        path: locate(step["remove"].as_str().unwrap()).unwrap(),
    }
}

#[test]
fn wire() {
    let wire = data();
    assert_eq!(wire["name"], "radicula.document.conformance/v1");
    assert_eq!(wire["version"], 1);
    let held = cases();
    let mut names: Vec<&str> = held
        .iter()
        .map(|case| case["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count);
    assert!(count >= 39);
}

#[test]
fn spec() {
    for case in cases() {
        let name = case["name"].as_str().unwrap();
        let actual = run(&case);
        let expect = case["expect"].as_object().unwrap();
        for (key, want) in expect {
            assert_eq!(actual.get(key), Some(want), "{name}: {key}");
        }
    }
}

#[test]
fn vocabulary() {
    let known = codes();
    for case in cases() {
        let Some(error) = case["expect"].get("error") else {
            continue;
        };
        assert!(known.contains(error), "{error} is not declared");
    }
    for case in cases() {
        let name = case["name"].as_str().unwrap();
        let actual = run(&case);
        if let Some(error) = actual.get("error") {
            assert!(known.contains(error), "{name}: {error} is not declared");
        }
    }
}
