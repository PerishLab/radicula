use std::collections::BTreeMap;

#[test]
fn expressions() {
    let vars = BTreeMap::from([("product", "plumb".to_string())]);
    let text = "image: forge\nrun: ${{ matrix.medium }} {@product}\njq: '{context: $held}'\n";
    assert_eq!(
        radicula_template::marked(text, &vars).expect("render"),
        "image: forge\nrun: ${{ matrix.medium }} plumb\njq: '{context: $held}'\n"
    );
}

#[test]
fn unknown() {
    let vars = BTreeMap::from([("product", "plumb".to_string())]);
    assert_eq!(
        radicula_template::marked("{@absent}", &vars),
        Err(radicula_template::Error::Unknown {
            name: "absent".to_string()
        })
    );
    assert_eq!(
        radicula_template::marked("{@open", &vars),
        Err(radicula_template::Error::Unclosed)
    );
}
