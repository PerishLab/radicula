use super::field::{self, N};
use super::point::Point;
use crate::{Error, ct};

fn admit(secret: &[u8; 32]) -> Result<(), Error> {
    let mut value = field::load(secret);
    let fine = !field::vanishes(&value).flag() && field::below(&value, &N.modulus());
    ct::wipe(&mut value);
    if fine { Ok(()) } else { Err(Error::Scalar) }
}

pub fn agree(secret: &[u8; 32], peer: &[u8]) -> Result<[u8; 32], Error> {
    admit(secret)?;
    let point = Point::decode(peer)?;
    let shared = point.mul(secret);
    let Some(mut abscissa) = shared.abscissa() else {
        return Err(Error::Zero);
    };
    let out = field::store(&abscissa);
    ct::wipe(&mut abscissa);
    Ok(out)
}

pub fn public(secret: &[u8; 32]) -> Result<[u8; 65], Error> {
    admit(secret)?;
    Point::base().mul(secret).encode()
}
