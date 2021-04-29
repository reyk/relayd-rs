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
}
