use std::{convert::Infallible, ops::Deref};

use thiserror::Error;

use crate::{
    protocols::onc_rpc::oncrpc_error::{RejectedReply, UnsuccessfulAcceptStatus},
    scpi,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("transfer layer error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("vxi11 protocol error: {0}")]
    Vxi11Error(#[from] super::protocols::onc_rpc::vxi11::vxi11_error::Vxi11Error),
    #[error("serial protocol error: {0}")]
    SerialError(#[from] serial::Error),
    #[error("scpi error: {0}")]
    ScpiError(#[from] scpi::scpi_error::ScpiError),
    #[error("one-rpc error: {0}")]
    OncRpcError(#[from] super::protocols::onc_rpc::oncrpc_error::OncRpcError),
    #[error("{0}")]
    Other(#[from] OtherError),
}
impl Error {
    pub fn is_timeout(&self) -> bool {
        if let Error::IOError(e) = self {
            if e.kind() == std::io::ErrorKind::TimedOut {
                return true;
            }
        }
        false
    }
}

#[derive(Debug)]
pub struct OtherError(String);

impl std::fmt::Display for OtherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error: {}", self.0)
    }
}

impl std::error::Error for OtherError {}
impl<'a> From<&'a str> for OtherError {
    fn from(s: &'a str) -> Self {
        Self(s.to_string())
    }
}
impl From<String> for OtherError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(s: &'a str) -> Self {
        Error::Other(OtherError::from(s))
    }
}
impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(OtherError::from(s))
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(s: std::string::FromUtf8Error) -> Self {
        Error::Other(OtherError(format!("{}", s.to_string())))
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(s: std::str::Utf8Error) -> Self {
        Error::Other(OtherError(format!("{}", s.to_string())))
    }
}

pub fn other_error<S: Deref<Target = str>>(s: S) -> OtherError {
    OtherError(s.to_string())
}
impl From<onc_rpc::Error> for Error {
    fn from(s: onc_rpc::Error) -> Self {
        Error::OncRpcError(s.into())
    }
}
impl From<RejectedReply> for Error {
    fn from(s: RejectedReply) -> Self {
        Error::OncRpcError(s.into())
    }
}
impl From<UnsuccessfulAcceptStatus> for Error {
    fn from(s: UnsuccessfulAcceptStatus) -> Self {
        Error::OncRpcError(s.into())
    }
}

impl From<serde_xdr::CompatSerializationError> for Error {
    fn from(s: serde_xdr::CompatSerializationError) -> Self {
        Error::OncRpcError(s.into())
    }
}
impl From<crate::protocols::onc_rpc::xdr::Error> for Error {
    fn from(s: crate::protocols::onc_rpc::xdr::Error) -> Self {
        Error::OncRpcError(s.into())
    }
}
impl From<Infallible> for Error {
    fn from(s: Infallible) -> Self {
        unreachable!()
    }
}
