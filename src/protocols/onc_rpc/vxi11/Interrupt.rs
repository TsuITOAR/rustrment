use crate::protocols::onc_rpc::{RpcProgram, RpcStream};

use super::{xdr, ErrorCode};
use bytes::{Bytes, BytesMut};
use std::io::Result;
pub enum Procedure {
    ///used by device to send a service request
    DeviceIntrSrq,
}

impl From<Procedure> for u32 {
    fn from(p: Procedure) -> Self {
        use Procedure::*;
        match p {
            DeviceIntrSrq => 30,
        }
    }
}

pub struct Interrupt<S> {
    io: S,
    buffer: BytesMut,
}

impl<S> RpcProgram for Interrupt<S> {
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

impl<S> Interrupt<S> {
    pub fn new(io: S) -> Self {
        Self {
            io,
            buffer: BytesMut::new(),
        }
    }
}

impl<S> Interrupt<S>
where
    S: RpcStream,
{
    ///receive interrupt message from device
    pub fn device_intr_srq(&mut self) -> Result<Bytes> {
        let buf = self.buffer();
        let mess = self.mut_io().read(buf)?;
        let call = mess.call_body().unwrap();
        debug_assert_eq!(call.program(), u32::from(Procedure::DeviceIntrSrq));
        debug_assert_eq!(call.program_version(), super::VERSION);
        Ok(call.payload().clone())
    }
}
