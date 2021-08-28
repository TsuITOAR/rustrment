use thiserror::Error;

use crate::{protocols, scpi};

#[derive(Error, Debug)]
enum Error {
    #[error("transfer layer error: {0}")]
    TransferError(#[from] std::io::Error),
    #[error("protocols error: {0}")]
    ProtocolError(#[from] protocols::protocol_error::ProtocolError),
    #[error("scpi error: {0}")]
    ScpiError(#[from] scpi::scpi_error::ScpiError),
}
