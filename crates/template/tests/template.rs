use radicula_template::{Error, Template};
use std::collections::BTreeMap;

#[test]
fn context() {
    let template = Template::new(
        "{{meta.user_agent}}/{{strand.id}}",
        &["meta.user_agent", "strand.id"],
    )
    .unwrap();
    for strand in ["ss_one", "ss_two"] {
        let vars = BTreeMap::from([
            ("meta.user_agent", "santi-api/v1.0.0".into()),
            ("strand.id", strand.into()),
        ]);
        assert_eq!(
            template.render(&vars).unwrap(),
            format!("santi-api/v1.0.0/{strand}")
        );
    }
}

#[test]
fn literal() {
    let vars = BTreeMap::from([("name", "{{other}}".into())]);
    for (text, expected) in [
        ("plain {text}", "plain {text}"),
        ("", ""),
        ("{{name}}", "{{other}}"),
    ] {
        assert_eq!(
            Template::new(text, &["name"])
                .unwrap()
                .render(&vars)
                .unwrap(),
            expected
        );
    }
}

#[test]
fn refusal() {
    for (text, expected) in [
        ("{{}}", Error::Empty),
        ("{{name}", Error::Unclosed),
        ("text}}", Error::Bare),
        (
            "{{ghost}}",
            Error::Unknown {
                name: "ghost".into(),
            },
        ),
        (
            "{{ name }}",
            Error::Unknown {
                name: " name ".into(),
            },
        ),
    ] {
        assert_eq!(Template::new(text, &["name"]).unwrap_err(), expected);
    }
    let template = Template::new("{{name}}", &["name"]).unwrap();
    assert_eq!(
        template.render(&BTreeMap::new()),
        Err(Error::Unknown {
            name: "name".into()
        })
    );
}
