use std::{fmt, option::NoneError};
pub enum Error {
    None(NoneError),
    /// Halt and Catch Fire
    Hcf,
    UnboundSymbol(String),
}

pub use Error::*;

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::None(_) => write!(f, "a none error, i guess"),
            Error::Hcf => write!(f, "halt and catch fire"),
            Error::UnboundSymbol(sym) => write!(f, "the symbol {} is unbound", sym),
        }
    }
}

// TODO: handle errors in a useful way, possibly with the `snafu` crate
impl From<NoneError> for Error {
    fn from(e: NoneError) -> Error {
        Error::None(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
