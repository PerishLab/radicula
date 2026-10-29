use crate::{Error, Failure};

pub(crate) trait Release {
    fn release(self) -> Result<(), Error>;
}

pub(crate) fn run<T>(permit: impl Release, action: impl FnOnce() -> T) -> Result<T, Failure<T>> {
    finish(permit, action())
}

pub(crate) fn finish<T>(permit: impl Release, output: T) -> Result<T, Failure<T>> {
    match permit.release() {
        Ok(()) => Ok(output),
        Err(error) => Err(Failure::Release { output, error }),
    }
}
