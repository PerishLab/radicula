use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Unknown { name: String },
    Unclosed,
    Bare,
    Empty,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown { name } => write!(f, "unknown variable {{{name}}}"),
            Self::Unclosed => write!(f, "unclosed {{"),
            Self::Bare => write!(f, "bare }} outside a variable"),
            Self::Empty => write!(f, "empty {{}}"),
        }
    }
}

impl std::error::Error for Error {}

pub fn fill(text: &str, vars: &BTreeMap<&str, String>) -> Result<String, Error> {
    let mut out = String::new();
    let mut rest = text.chars().peekable();
    while let Some(mark) = rest.next() {
        match mark {
            '{' => open(&mut out, &mut rest, vars)?,
            '}' => close(&mut out, &mut rest)?,
            held => out.push(held),
        }
    }
    Ok(out)
}

pub fn marked(text: &str, vars: &BTreeMap<&str, String>) -> Result<String, Error> {
    let mut out = String::new();
    let mut rest = text.chars().peekable();
    while let Some(mark) = rest.next() {
        if mark != '{' || rest.peek() != Some(&'@') {
            out.push(mark);
            continue;
        }
        rest.next();
        let mut name = String::new();
        let mut closed = false;
        for held in rest.by_ref() {
            if held == '}' {
                closed = true;
                break;
            }
            name.push(held);
        }
        if !closed {
            return Err(Error::Unclosed);
        }
        resolve(&mut out, &name, vars)?;
    }
    Ok(out)
}

fn open(
    out: &mut String,
    rest: &mut std::iter::Peekable<std::str::Chars<'_>>,
    vars: &BTreeMap<&str, String>,
) -> Result<(), Error> {
    if rest.peek() == Some(&'{') {
        rest.next();
        out.push('{');
        return Ok(());
    }
    let mut name = String::new();
    for held in rest.by_ref() {
        if held == '}' {
            return resolve(out, &name, vars);
        }
        name.push(held);
    }
    Err(Error::Unclosed)
}

fn close(
    out: &mut String,
    rest: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<(), Error> {
    if rest.peek() == Some(&'}') {
        rest.next();
        out.push('}');
        return Ok(());
    }
    Err(Error::Bare)
}

fn resolve(out: &mut String, name: &str, vars: &BTreeMap<&str, String>) -> Result<(), Error> {
    if name.is_empty() {
        return Err(Error::Empty);
    }
    match vars.get(name) {
        Some(value) => {
            out.push_str(value);
            Ok(())
        }
        None => Err(Error::Unknown { name: name.into() }),
    }
}
