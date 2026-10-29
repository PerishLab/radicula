use super::{key::Key, lease::Fence, wire::Reply};
use crate::Error;
use std::{io, sync::Arc};

pub(super) fn admission(reply: Reply, key: &Key, expected: &str) -> Result<Option<Fence>, Error> {
    let decoded = decode(reply, key, expected);
    match decoded {
        Err(Error::Io(error)) => {
            key.fault("Redis admission reply is invalid; no execution may start");
            Err(Error::Unconfirmed(error))
        }
        Err(Error::Epoch) => {
            key.fault("Redis server epoch changed; explicit reconstruction is required");
            Err(Error::Epoch)
        }
        other => other,
    }
}

fn decode(reply: Reply, key: &Key, expected: &str) -> Result<Option<Fence>, Error> {
    let Reply::Array(fields) = reply else {
        return Err(invalid());
    };
    match fields.as_slice() {
        [Reply::Integer(0)] => Ok(None),
        [Reply::Integer(-2)] => Err(Error::Epoch),
        [Reply::Integer(-3)] => Err(Error::Conflict(key.name.clone())),
        [Reply::Integer(1), Reply::Text(epoch), Reply::Text(sequence)] => {
            let epoch = std::str::from_utf8(epoch).map_err(|_| invalid())?;
            let sequence = std::str::from_utf8(sequence)
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|value| *value > 0)
                .ok_or_else(invalid)?;
            if epoch != expected {
                return Err(Error::Epoch);
            }
            Ok(Some(Fence {
                epoch: epoch.into(),
                sequence,
            }))
        }
        _ => Err(invalid()),
    }
}

fn invalid() -> Error {
    Error::Io(Arc::new(io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid Redis admission reply",
    )))
}
