use std::fmt::{Display, Formatter};
use core::fmt;

pub(crate) type Result<T> = std::result::Result<T, Error>;
pub(crate) type Operation = Result<()>;

pub enum Error {
    Err(Box<String>)
}

impl Error {
    pub fn new<S: AsRef<str> + ?Sized>(msg: &S) -> Error {
        Error::Err(Box::from(String::from(msg.as_ref())))
    }
}


impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Err(msg) => write!(f, "{}", msg)
        }
    }
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        e.as_ox_error()
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        e.as_ox_error()
    }
}

pub trait OxError {
    fn as_ox_error(&self) -> Error;
}

pub trait OxResult<T, E> {
    fn into_ox_result(self) -> Result<T>;
}

impl OxError for git2::Error {
    fn as_ox_error(&self) -> Error {
        Error::Err(Box::from(self.message().to_string()))
    }
}

impl OxError for std::io::Error {
    fn as_ox_error(&self) -> Error {
        Error::Err(Box::from(self.to_string()))
    }
}

impl<T, E> OxResult<T, E> for core::result::Result<T, E> where E: OxError {
    fn into_ox_result(self) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.as_ox_error()),
        }
    }
}
