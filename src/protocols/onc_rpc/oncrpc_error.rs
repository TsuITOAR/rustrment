use onc_rpc::AcceptedStatus;
use std::convert::Infallible;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OncRpcError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("invalid onc-rpc message: {0}")]
    InvalidRpcMessage(#[from] onc_rpc::Error),
    #[error("onc-rpc rejected: {0}")]
    RpcRejected(#[from] RejectedReply),
    #[error("onc-rpc accepted status error: {0}")]
    UnsuccessfulAcceptStatus(#[from] UnsuccessfulAcceptStatus),
    #[error("serialize procedure specific parameters error: {0}")]
    SerializationError(#[from] serde_xdr::CompatSerializationError),
    #[error("deserialize procedure specific parameters error: {0}")]
    DeserializationError(#[from] super::xdr::Error),
    #[error("reply xid unmatched, expected {0}, found {1}")]
    XidUnmatched(u32, u32),
    #[error("rpc error: '{0}'")]
    Other(String),
}
impl OncRpcError {
    pub fn is_timeout(&self) -> bool {
        if let OncRpcError::IOError(e) = self {
            if e.kind() == std::io::ErrorKind::TimedOut {
                return true;
            }
        }
        false
    }
}

impl From<Infallible> for OncRpcError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Error, Debug)]
pub enum UnsuccessfulAcceptStatus {
    #[error("The specified program number has no handler in this server.")]
    ProgramUnavailable,
    #[error("The program to invoke was found, but it doesn’t support the requested version, supported version: {low}-{high}.")]
    ProgramMismatch { low: u32, high: u32 },
    #[error("The program to invoke was found, but the procedure number is not recognized.")]
    ProcedureUnavailable,
    #[error("The arguments provided to the RPC endpoint were not serialized correctly, or otherwise unacceptable.")]
    GarbageArgs,
    #[error("The server experienced an internal error.")]
    SystemError,
}

pub(crate) struct PrivateWrapper<'a, S: AsRef<[u8]>>(pub(crate) &'a AcceptedStatus<S>);
impl<'a, S: AsRef<[u8]>> From<PrivateWrapper<'a, S>> for UnsuccessfulAcceptStatus {
    fn from(value: PrivateWrapper<'a, S>) -> Self {
        let value = value.0;
        match value {
            AcceptedStatus::Success(_) => unreachable!(),
            AcceptedStatus::ProgramUnavailable => UnsuccessfulAcceptStatus::ProgramUnavailable,
            AcceptedStatus::ProgramMismatch { low, high } => {
                UnsuccessfulAcceptStatus::ProgramMismatch {
                    low: *low,
                    high: *high,
                }
            }

            AcceptedStatus::ProcedureUnavailable => UnsuccessfulAcceptStatus::ProcedureUnavailable,
            AcceptedStatus::GarbageArgs => UnsuccessfulAcceptStatus::GarbageArgs,
            AcceptedStatus::SystemError => UnsuccessfulAcceptStatus::SystemError,
        }
    }
}

#[derive(Error, Debug)]
pub enum RejectedReply {
    #[error("The RPC version was not serviceable, supported version: {low}-{high}")]
    RpcVersionMismatch { low: u32, high: u32 },
    #[error("The authentication credentials included in the request (if any) were rejected: {0}")]
    AuthError(String),
}

impl From<&onc_rpc::RejectedReply> for RejectedReply {
    fn from(value: &onc_rpc::RejectedReply) -> Self {
        match value {
            onc_rpc::RejectedReply::RpcVersionMismatch { low, high } => {
                RejectedReply::RpcVersionMismatch {
                    low: *low,
                    high: *high,
                }
            }
            onc_rpc::RejectedReply::AuthError(s) => RejectedReply::AuthError(format!("{:?}", s)),
        }
    }
}
