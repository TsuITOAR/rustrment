use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("vxi11 protocol error: {0}")]
    Vxi11Error(#[from] super::onc_rpc::vxi11::error::Vxi11Error),
    #[error("serial protocol error: {0}")]
    SerialError(#[from] serial::Error)
}
