//! custom error

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Error(String),
}

impl Error {
    pub(crate) fn convert_string(str: &str) -> String {
        return Error::Error(str.to_string()).to_string();
    }
}
