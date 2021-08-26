use super::xdr;
pub mod abort;
pub mod core;
pub mod interrupt;

const VERSION: u32 = 1;


fn error_to_i32(l: xdr::Device_ErrorCode) -> i32 {
    (l.0).0
}
pub enum ErrorCode {
    ///No error
    NoError,
    ///Syntax error
    SyntaxError,
    ///device not accessible
    NotAccessible,
    ///invalid link identifier
    InvalidIdentifier,
    ///parameter error
    ParameterError,
    ///channel not established
    NotEstablished,
    ///operation not supported
    NotSupported,
    ///out of resources
    OutOfResources,
    ///device locked by another link
    LockedByAnother,
    ///no lock held by this link
    NoLockHeld,
    ///I/O timeout
    IOTimeOut,
    ///I/O error
    IOError,
    ///Invalid address
    InvalidAddress,
    ///abort
    Abort,
    ///channel already established
    AlreadyEstablished,
    ///Unknown error code
    Unknown(i32),
}
impl ToString for ErrorCode {
    fn to_string(&self) -> String {
        use ErrorCode::*;
        match self {
            NoError => "no error",
            SyntaxError => "syntax error",
            NotAccessible => "device not accessible",
            InvalidIdentifier => "invalid link identifier",
            ParameterError => "parameter error",
            NotEstablished => "channel not established",
            NotSupported => "operation not supported",
            OutOfResources => "out of resources",
            LockedByAnother => "device locked by another link",
            NoLockHeld => "no lock held by this link",
            IOTimeOut => "I/O timeout",
            IOError => "I/O error",
            InvalidAddress => "Invalid address",
            Abort => "abort",
            AlreadyEstablished => "channel already established",
            Unknown(s) => return format!("Unknown error code: {}", s),
        }
        .into()
    }
}
impl From<xdr::Device_ErrorCode> for ErrorCode {
    fn from(e: xdr::Device_ErrorCode) -> Self {
        use ErrorCode::*;
        let i: i32 = error_to_i32(e);
        match i {
            0 => NoError,
            1 => SyntaxError,
            3 => NotAccessible,
            4 => InvalidIdentifier,
            5 => ParameterError,
            6 => NotEstablished,
            8 => NotSupported,
            9 => OutOfResources,
            11 => LockedByAnother,
            12 => NoLockHeld,
            15 => IOTimeOut,
            17 => IOError,
            21 => InvalidAddress,
            23 => Abort,
            29 => AlreadyEstablished,
            n => Unknown(n),
        }
    }
}

impl From<xdr::Device_Error> for ErrorCode {
    fn from(e: xdr::Device_Error) -> Self {
        Self::from(e.error)
    }
}
impl From<ErrorCode> for std::io::Result<()> {
    fn from(e: ErrorCode) -> Self {
        use std::io::{Error, ErrorKind};
        use ErrorCode::*;
        match e {
            NoError => Ok(()),
            other => Err(Error::new(ErrorKind::Other, other.to_string())),
        }
    }
}
