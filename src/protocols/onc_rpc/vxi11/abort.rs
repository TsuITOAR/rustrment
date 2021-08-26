use super::{xdr, ErrorCode};
use bytes::BytesMut;
use std::{io::Result, net::TcpStream};

use crate::protocols::onc_rpc::{Rpc, RpcProgram};

pub enum Procedure {
    ///device aborts an in-progress call
    DeviceAbort,
}

impl From<Procedure> for u32 {
    fn from(p: Procedure) -> Self {
        use Procedure::*;
        match p {
            DeviceAbort => 1,
        }
    }
}

pub struct Abort<S> {
    io: S,
    buffer: BytesMut,
}

impl<S> RpcProgram for Abort<S> {
    type IO = S;
    const PROGRAM: u32 = 0x0607AF;
    const VERSION: u32 = super::VERSION;
    fn get_io(&self) -> &Self::IO {
        &self.io
    }
    fn mut_io(&mut self) -> &mut Self::IO {
        &mut self.io
    }
    fn buffer(&self) -> BytesMut {
        self.buffer.clone()
    }
}

impl<S> Abort<S> {
    pub fn new(io: S) -> Self {
        Self {
            io,
            buffer: BytesMut::new(),
        }
    }
}

impl Abort<TcpStream> {
    pub fn device_abort(&mut self, link_id: i32) -> Result<()> {
        let resp: xdr::Device_Error =
            self.call_anonymously(Procedure::DeviceAbort, xdr::Device_Link(xdr::long(link_id)))?;
        Result::from(ErrorCode::from(resp))
    }
}
