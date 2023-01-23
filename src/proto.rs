use self::response::Response;

pub mod codec;
pub mod command;
pub mod device;
pub mod response;

#[cfg(test)]
pub mod fake;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtoError {
    #[error("I/O error: {:?}", _0)]
    Io(#[from] std::io::Error),

    #[error("Serial I/O error: {:?}", _0)]
    Serial(#[from] tokio_serial::Error),

    #[error("Command was invalid or contains syntax errors")]
    SyntaxError,
    #[error("Execution error")]
    ExecutionError,
    #[error("Connection was closed")]
    Abort,
    #[error("Unexpected response: {:?}", _0)]
    Unexpected(Response),
}

impl From<Response> for ProtoError {
    fn from(value: Response) -> Self {
        match value {
            Response::SyntaxError => Self::SyntaxError,
            Response::ExecutionError => Self::ExecutionError,
            Response::Success(_) => Self::Unexpected(value),
            Response::NoData => Self::Unexpected(value),
        }
    }
}

pub type Result<T> = std::result::Result<T, ProtoError>;
