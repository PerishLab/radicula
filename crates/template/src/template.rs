use crate::Error;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
enum Part {
    Text(String),
    Variable(String),
}

#[derive(Debug, Clone)]
pub struct Template(Vec<Part>);

impl Template {
    pub fn new(text: &str, names: &[&str]) -> Result<Self, Error> {
        let mut parts = Vec::new();
        let mut rest = text;
        while let Some(start) = rest.find("{{") {
            Self::text(&mut parts, &rest[..start])?;
            rest = &rest[start + 2..];
            let end = rest.find("}}").ok_or(Error::Unclosed)?;
            let name = &rest[..end];
            if name.is_empty() {
                return Err(Error::Empty);
            }
            if !names.contains(&name) {
                return Err(Error::Unknown { name: name.into() });
            }
            parts.push(Part::Variable(name.into()));
            rest = &rest[end + 2..];
        }
        Self::text(&mut parts, rest)?;
        Ok(Self(parts))
    }

    pub fn render(&self, vars: &BTreeMap<&str, String>) -> Result<String, Error> {
        let mut out = String::new();
        for part in &self.0 {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::Variable(name) => {
                    let value = vars
                        .get(name.as_str())
                        .ok_or_else(|| Error::Unknown { name: name.clone() })?;
                    out.push_str(value);
                }
            }
        }
        Ok(out)
    }

    fn text(parts: &mut Vec<Part>, text: &str) -> Result<(), Error> {
        if text.contains("}}") {
            return Err(Error::Bare);
        }
        if !text.is_empty() {
            parts.push(Part::Text(text.into()));
        }
        Ok(())
    }
}
