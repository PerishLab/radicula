use crate::{Error, Search, Tool};
use std::ffi::{OsStr, OsString};
use std::process::Command;

#[derive(Clone)]
pub struct Binding {
    search: Search,
    program: OsString,
    tool: Tool,
}

impl Binding {
    pub(crate) fn new(search: Search, program: &OsStr) -> Result<Self, Error> {
        let tool = Tool::at(search.resolve(program)?)?;
        Ok(Self {
            search,
            program: program.to_owned(),
            tool,
        })
    }

    pub fn tool(&self) -> &Tool {
        &self.tool
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.search.resolve(&self.program)? != self.tool.path {
            return Err(Error::Changed(self.tool.path.clone()));
        }
        self.tool.verify()
    }

    pub fn command(&self) -> Result<Command, Error> {
        self.verify()?;
        Ok(self.search.at(&self.tool.path))
    }
}
