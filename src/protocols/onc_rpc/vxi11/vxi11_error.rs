use thiserror::Error;

#[derive(Error, Debug)]
pub enum Vxi11Error {
    #[error("syntax error")]
    SyntaxError,
    #[error("device not accessible")]
    NotAccessible,
    #[error("invalid link identifier")]
    InvalidIdentifier,
    #[error("parameter error")]
    ParameterError,
    #[error("channel not established")]
    NotEstablished,
    #[error("operation not supported")]
    NotSupported,
    #[error("out of resources")]
    OutOfResources,
    #[error("device locked by another link")]
    LockedByAnother,
    #[error("no lock held by this link")]
    NoLockHeld,
    #[error("I/O timeout")]
    IOTimeOut,
    #[error("I/O error")]
    IOError,
    #[error("invalid address")]
    InvalidAddress,
    #[error("abort")]
    Abort,
    #[error("channel already established")]
    AlreadyEstablished,
    #[error("unknown vxi11 error code: '{0}'")]
    Vxi11Unknown(i32),
    #[error("one-rpc error: {0}")]
    OncRpcError(#[from] super::super::oncrpc_error::OncRpcError),
    #[error("parse socket adderss error: {0}")]
    InvalidSocketAddr(#[from] std::io::Error),
    #[error("device output buffer full")]
    DevOutputBufFull
}
