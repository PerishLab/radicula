use crate::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    parts: Vec<String>,
}

impl Path {
    pub fn root() -> Self {
        Self { parts: Vec::new() }
    }

    pub fn parse(text: &str) -> Result<Self, Error> {
        if text.is_empty() {
            return Ok(Self::root());
        }
        let rest = text.strip_prefix('/').ok_or(Error::Pointer)?;
        let mut parts = Vec::new();
        for part in rest.split('/') {
            parts.push(decode(part)?);
        }
        Ok(Self { parts })
    }

    pub fn parts(&self) -> &[String] {
        &self.parts
    }
}

fn decode(text: &str) -> Result<String, Error> {
    let mut out = String::new();
    let mut rest = text.chars();
    while let Some(mark) = rest.next() {
        if mark != '~' {
            out.push(mark);
            continue;
        }
        match rest.next() {
            Some('0') => out.push('~'),
            Some('1') => out.push('/'),
            _ => return Err(Error::Escape),
        }
    }
    Ok(out)
}
