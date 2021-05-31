use derive_more::{Display, From};
use std::io;

/// Common errors of the `privsep` crate.
#[derive(Debug, Display, From)]
pub enum Error {
    #[display(fmt = "I/O error: {}", "_0")]
    IoError(io::Error),
    #[display(fmt = "Invalid arguments: {}", "_0")]
    Options(getopts::Fail),
    #[display(fmt = "Privilge separation error: {}", "_0")]
    PrivsepError(privsep::Error),
    #[display(fmt = "Parser error: {}", "_0")]
    ParserError(String),
    #[display(fmt = "Lost {}, terminated", "_0")]
    #[from(ignore)]
    Terminated(&'static str),
}

impl std::error::Error for Error {}

// Convert to privsep error.
impl From<Error> for privsep::Error {
    fn from(error: Error) -> privsep::Error {
        privsep::Error::GeneralError(Box::new(error))
    }
}
