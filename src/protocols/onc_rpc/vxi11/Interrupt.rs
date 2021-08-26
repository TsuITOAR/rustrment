use crate::protocols::onc_rpc::{RpcProgram, RpcStream};

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
    prog_num: u32,
    prog_ver: u32,
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
    pub fn new(prog_num: u32, prog_ver: u32, io: S) -> Self {
        Self {
            prog_num,
            prog_ver,
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
        loop {
            let buf = self.buffer();
            let mess = self.mut_io().read(buf)?;
            let call = mess.call_body().unwrap();
            if call.program() == self.prog_num
                && call.program_version() == self.prog_ver
                && call.procedure() == u32::from(Procedure::DeviceIntrSrq)
            {
                return Ok(call.payload().clone());
            }
        }
    }
}
