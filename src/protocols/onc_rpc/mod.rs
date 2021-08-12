mod port_mapper;
mod xdr;
use onc_rpc::{AcceptedReply, CallBody, MessageType, ReplyBody, RpcMessage};
use std::io::{Read, Result, Write};
pub trait OncRpc {
    const PROGRAM: u32;
    const VERSION: u32;
    type Procedure: Into<u32>;
    type Writer: std::io::Write;
    type Reader: std::io::Read;
    fn writer(&mut self) -> &mut Self::Writer;
    fn reader(&mut self) -> &mut Self::Reader;
    fn call<'a, T, P>(
        &mut self,
        xid: u32,
        call_body: CallBody<T, P>,
        buf: &'a mut Vec<u8>,
    ) -> Result<RpcMessage<&'a [u8], &'a [u8]>>
    where
        T: AsRef<[u8]>,
        P: AsRef<[u8]>,
    {
        raw_rpc_write(
            self.writer(),
            &RpcMessage::new(xid, MessageType::Call(call_body)).serialise()?,
        )?;
        buf.clear();
        let reply = raw_rpc_read(self.reader(), buf)?;
        if reply.xid() == xid {
            Ok(reply)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unmatched xid, expected {}, got {}", xid, reply.xid(),),
            ))
        }
    }
    fn reply<T, P>(&mut self, xid: u32, reply_body: ReplyBody<T, P>) -> Result<()>
    where
        T: AsRef<[u8]>,
        P: AsRef<[u8]>,
    {
        raw_rpc_write(
            self.writer(),
            &RpcMessage::new(xid, MessageType::Reply(reply_body)).serialise()?,
        )
    }
    fn read_call<'a>(&mut self, buf: &'a mut Vec<u8>) -> Result<RpcMessage<&'a [u8], &'a [u8]>> {
        raw_rpc_read(self.reader(), buf)
    }
}

fn raw_rpc_write<W: Write, M: AsRef<[u8]>>(writer: &mut W, message: &M) -> Result<()> {
    writer.write_all(message.as_ref())
}
fn raw_rpc_read<'a, R: Read>(
    reader: &mut R,
    buf: &'a mut Vec<u8>,
) -> Result<RpcMessage<&'a [u8], &'a [u8]>> {
    buf.clear();
    let mut header = [0_u8; 4];
    reader.read_exact(&mut header)?;
    let body_len: u32 =
        u32::from_be_bytes([header[0] & (!(1 << 4)), header[1], header[2], header[3]]);
    buf.extend_from_slice(&header);
    buf.resize(body_len as usize + 4, 0);
    reader.read_exact(&mut buf[5..])?;
    match RpcMessage::from_bytes(&buf[..]) {
        Ok(message) => Ok(message),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    }
}
