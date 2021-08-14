mod port_mapper;
mod xdr;
use bytes::{Bytes, BytesMut};
use onc_rpc::{CallBody, MessageType, ReplyBody, RpcMessage};
use std::{
    convert::TryFrom,
    io::{Read, Result, Write},
};

const HEAD_LEN: usize = 4;

pub struct OncResult(pub Bytes);
impl From<Bytes> for OncResult {
    fn from(b: Bytes) -> Self {
        Self(b)
    }
}
pub trait OncRpc {
    const PROGRAM: u32;
    const VERSION: u32;
    type Procedure: Into<u32>;
    type Writer: std::io::Write;
    type Reader: std::io::Read;
    fn writer(&mut self) -> &mut Self::Writer;
    fn reader(&mut self) -> &mut Self::Reader;
    fn buffer(&mut self) -> BytesMut;
    fn call<'a, T, P>(&'a mut self, xid: u32, call_body: CallBody<T, P>) -> Result<OncResult>
    where
        T: AsRef<[u8]>,
        P: AsRef<[u8]>,
    {
        raw_rpc_write(
            self.writer(),
            &RpcMessage::new(xid, MessageType::Call(call_body)).serialise()?,
        )?;
        let reply = self.read()?;
        if reply.xid() == xid {
            match reply.reply_body().ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "expected reply, found call",
            ))? {
                ReplyBody::Accepted(a) => match a.status() {
                    onc_rpc::AcceptedStatus::Success(p) => Ok(p.clone().into()),

                    onc_rpc::AcceptedStatus::ProgramUnavailable => Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "program unavailable",
                    )),
                    onc_rpc::AcceptedStatus::ProgramMismatch { low, high } => {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("program mismatch, supported version {} - {}", low, high),
                        ))
                    }
                    onc_rpc::AcceptedStatus::ProcedureUnavailable => Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "procedure unavailable",
                    )),
                    onc_rpc::AcceptedStatus::GarbageArgs => Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "garbage args",
                    )),
                    onc_rpc::AcceptedStatus::SystemError => Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "system error",
                    )),
                },
                ReplyBody::Denied(d) => match d {
                    onc_rpc::RejectedReply::RpcVersionMismatch { low, high } => {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("rpc version mismatch, supported version {} - {}", low, high),
                        ))
                    }
                    onc_rpc::RejectedReply::AuthError(a) => Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("authentication failed, {:?}", a),
                    )),
                },
            }
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
    fn read(&mut self) -> Result<RpcMessage<Bytes, Bytes>> {
        let buf = self.buffer().clone();
        Ok(parse_bytes(raw_rpc_read(self.reader(), buf)?)?)
    }
}

fn parse_bytes(bytes: Bytes) -> Result<RpcMessage<Bytes, Bytes>> {
    match RpcMessage::try_from(bytes) {
        Ok(m) => Ok(m),
        Err(onc_rpc::Error::IncompleteHeader) | Err(onc_rpc::Error::IncompleteMessage { .. }) => {
            unreachable!()
        }
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    }
}

fn raw_rpc_write<W: Write, M: AsRef<[u8]>>(writer: &mut W, message: &M) -> Result<()> {
    writer.write_all(message.as_ref())
}

fn raw_rpc_read<'a, R: Read>(reader: &mut R, mut buf: BytesMut) -> Result<Bytes> {
    assert!(buf.is_empty(), "expect buffer empty");
    buf.reserve(HEAD_LEN);
    loop {
        reader.read(buf.as_mut())?;

        let expected_len =
            match onc_rpc::expected_message_len(buf.clone().as_ref() /* zero-copy clone */) {
                Ok(len) => len as usize,
                Err(onc_rpc::Error::IncompleteHeader) => continue,
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            };

        if expected_len > buf.len() {
            buf.reserve(expected_len - buf.len());
            // The buffer does not contain a full message, read more data
            continue;
        } else {
            // Split the buffer into a single message
            let msg_bytes = buf.split_to(expected_len as usize);

            return Ok(msg_bytes.freeze());
        }
    }
}
