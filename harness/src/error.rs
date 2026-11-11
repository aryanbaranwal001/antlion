use std::fmt;

/// Why antlion stopped: a wrong command, or anything else.
#[derive(Debug)]
pub enum Error {
    Usage(String),
    Failed(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn usage(msg: impl Into<String>) -> Self {
        Error::Usage(msg.into())
    }

    pub fn failed(msg: impl Into<String>) -> Self {
        Error::Failed(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (Error::Usage(m) | Error::Failed(m)) = self;
        write!(f, "[err] {m}")
    }
}
