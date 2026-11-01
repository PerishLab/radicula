use crate::{Error, output::decode};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Type,
    Unmerged,
    Unknown,
    Broken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    pub status: Status,
    pub score: Option<u8>,
    pub path: PathBuf,
    pub previous: Option<PathBuf>,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<Change>, Error> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let bytes = bytes
        .strip_suffix(&[0])
        .ok_or(Error::Invalid("NUL framing"))?;
    let mut fields = bytes.split(|b| *b == 0);
    let mut changes = Vec::new();
    while let Some(status) = fields.next() {
        changes.push(record(status, &mut fields)?);
    }
    Ok(changes)
}

fn record<'a>(status: &[u8], fields: &mut impl Iterator<Item = &'a [u8]>) -> Result<Change, Error> {
    let (status, score) = kind(status)?;
    let first = next(fields)?;
    let (previous, path) = match status {
        Status::Renamed | Status::Copied => (Some(first), next(fields)?),
        _ => (None, first),
    };
    Ok(Change {
        status,
        score,
        path,
        previous,
    })
}

fn next<'a>(fields: &mut impl Iterator<Item = &'a [u8]>) -> Result<PathBuf, Error> {
    decode(
        fields
            .next()
            .ok_or(Error::Invalid("missing path"))?
            .to_vec(),
    )
}

fn kind(bytes: &[u8]) -> Result<(Status, Option<u8>), Error> {
    let (tag, rest) = bytes.split_first().ok_or(Error::Invalid("empty status"))?;
    let status = match tag {
        b'A' => Status::Added,
        b'M' => Status::Modified,
        b'D' => Status::Deleted,
        b'R' => Status::Renamed,
        b'C' => Status::Copied,
        b'T' => Status::Type,
        b'U' => Status::Unmerged,
        b'X' => Status::Unknown,
        b'B' => Status::Broken,
        _ => return Err(Error::Invalid("change status")),
    };
    let scored = matches!(status, Status::Renamed | Status::Copied | Status::Modified);
    if !scored && !rest.is_empty() {
        return Err(Error::Invalid("unexpected score"));
    }
    let score = score(rest)?;
    if matches!(status, Status::Renamed | Status::Copied) && score.is_none() {
        return Err(Error::Invalid("missing score"));
    }
    Ok((status, score))
}

fn score(bytes: &[u8]) -> Result<Option<u8>, Error> {
    if bytes.is_empty() {
        return Ok(None);
    }
    if !bytes.iter().all(u8::is_ascii_digit) {
        return Err(Error::Invalid("score"));
    }
    let score = std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| text.parse::<u8>().ok())
        .filter(|score| *score <= 100)
        .ok_or(Error::Invalid("score"))?;
    Ok(Some(score))
}
