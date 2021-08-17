pub mod port_mapper;
use bytes::{Bytes, BytesMut};
use onc_rpc::{auth::AuthFlavor, CallBody, MessageType, ReplyBody, RpcMessage};
use std::{
    convert::TryFrom,
    io::{Error, ErrorKind, Result},
    net::{SocketAddr, ToSocketAddrs},
};
include!(concat!(env!("OUT_DIR"), r#"/xdr.rs"#));

const HEAD_LEN: usize = 4;
fn parse_bytes(bytes: Bytes) -> Result<RpcMessage<Bytes, Bytes>> {
    match RpcMessage::try_from(bytes) {
        Ok(m) => Ok(m),
        Err(onc_rpc::Error::IncompleteHeader) | Err(onc_rpc::Error::IncompleteMessage { .. }) => {
            unreachable!()
        }
        Err(e) => Err(Error::new(ErrorKind::InvalidData, e)),
    }
}
pub trait OncRpc {
    const PROGRAM: u32;
    const VERSION: u32;
    type Procedure: Into<u32>;
    fn raw_write(&mut self, buf: &[u8]) -> Result<usize>;
    fn raw_read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn buffer(&self) -> BytesMut;

    fn gen_xid(&mut self) -> u32 {
        rand::random()
    }

    fn send<T, P>(&mut self, message: RpcMessage<T, P>) -> Result<()>
    where
        T: AsRef<[u8]>,
        P: AsRef<[u8]>,
    {
        raw_write_all(self, &message.serialise()?)
    }

    fn read(&mut self) -> Result<RpcMessage<Bytes, Bytes>> {
        let mut buf = self.buffer().clone();
        if buf.len() < HEAD_LEN {
            buf.reserve(HEAD_LEN - buf.len())
        };
        let bytes = loop {
            self.raw_read(buf.as_mut())?;

            let expected_len =
                match onc_rpc::expected_message_len(buf.clone().as_ref() /* zero-copy clone */) {
                    Ok(len) => len as usize,
                    Err(onc_rpc::Error::IncompleteHeader) => continue,
                    Err(e) => return Err(Error::new(ErrorKind::InvalidData, e)),
                };

            if expected_len > buf.len() {
                buf.reserve(expected_len - buf.len());
                // The buffer does not contain a full message, read more data
                continue;
            } else {
                // Split the buffer into a single message
                let msg_bytes = buf.split_to(expected_len as usize);
                break Result::Ok(msg_bytes.freeze());
            }
        }?;
        Ok(parse_bytes(bytes)?)
    }
    fn call<T: AsRef<[u8]>, C: AsRef<[u8]>>(
        &mut self,
        procedure: Self::Procedure,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: &C,
    ) -> Result<Bytes> {
        let xid = self.gen_xid();
        let call_body = CallBody::new(
            Self::PROGRAM,
            Self::VERSION,
            procedure.into(),
            auth_credentials,
            auth_verifier,
            content.as_ref(),
        );
        self.raw_write(&RpcMessage::new(xid, MessageType::Call(call_body)).serialise()?)?;
        let reply = self.read()?;
        if reply.xid() == xid {
            match reply.reply_body().ok_or(Error::new(
                ErrorKind::InvalidInput,
                "expected reply, found call",
            ))? {
                ReplyBody::Accepted(a) => match a.status() {
                    onc_rpc::AcceptedStatus::Success(p) => Ok(p.clone().into()),

                    onc_rpc::AcceptedStatus::ProgramUnavailable => {
                        Err(Error::new(ErrorKind::Other, "program unavailable"))
                    }
                    onc_rpc::AcceptedStatus::ProgramMismatch { low, high } => Err(Error::new(
                        ErrorKind::Other,
                        format!("program mismatch, supported version {} - {}", low, high),
                    )),
                    onc_rpc::AcceptedStatus::ProcedureUnavailable => {
                        Err(Error::new(ErrorKind::Other, "procedure unavailable"))
                    }
                    onc_rpc::AcceptedStatus::GarbageArgs => {
                        Err(Error::new(ErrorKind::Other, "garbage args"))
                    }
                    onc_rpc::AcceptedStatus::SystemError => {
                        Err(Error::new(ErrorKind::Other, "system error"))
                    }
                },
                ReplyBody::Denied(d) => match d {
                    onc_rpc::RejectedReply::RpcVersionMismatch { low, high } => Err(Error::new(
                        ErrorKind::Other,
                        format!("rpc version mismatch, supported version {} - {}", low, high),
                    )),
                    onc_rpc::RejectedReply::AuthError(a) => Err(Error::new(
                        ErrorKind::PermissionDenied,
                        format!("authentication failed, {:?}", a),
                    )),
                },
            }
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                format!("unmatched xid, expected {}, got {}", xid, reply.xid(),),
            ))
        }
    }
    fn call_anonymously<C: AsRef<[u8]>>(
        &mut self,
        procedure: Self::Procedure,
        content: &C,
    ) -> Result<Bytes> {
        self.call::<&[u8], C>(
            procedure,
            AuthFlavor::AuthNone(None),
            AuthFlavor::AuthNone(None),
            content,
        )
    }
}

pub trait OncRpcBroadcast: OncRpc {
    fn raw_send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> Result<usize>;
    fn raw_recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>;
    fn recv_from(&mut self) -> Result<(RpcMessage<Bytes, Bytes>, SocketAddr)> {
        let mut buf = self.buffer().clone();
        if buf.len() < HEAD_LEN {
            buf.reserve(HEAD_LEN - buf.len())
        };
        let (bytes, addr) = {
            //UDP don't fragment
            let (_, addr) = self.raw_recv_from(buf.as_mut())?;

            let expected_len =
                match onc_rpc::expected_message_len(buf.clone().as_ref() /* zero-copy clone */) {
                    Ok(len) => len as usize,
                    Err(e) => return Err(Error::new(ErrorKind::InvalidData, e)),
                };
            let len = buf.len();
            if expected_len > len {
                buf.reserve(expected_len - len);
                // The buffer does not contain a full message, read more data
                raw_read_exact(self, &mut buf.as_mut()[..expected_len - len])?;
            }

            // Split the buffer into a single message
            let msg_bytes = buf.split_to(expected_len as usize);
            (msg_bytes.freeze(), addr)
        };
        Ok((parse_bytes(bytes)?, addr))
    }
    fn send_to<A: ToSocketAddrs, T: AsRef<[u8]>, P: AsRef<[u8]>>(
        &self,
        message: RpcMessage<T, P>,
        addr: A,
    ) -> Result<()> {
        let buf = message.serialise()?;
        if !buf.is_empty() {
            match self.raw_send_to(&buf, addr) {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) if n == buf.len() => return Ok(()),
                Ok(n) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!(
                            "only {} byte(s) message sent, expected {} bytes",
                            n,
                            buf.len()
                        ),
                    ))
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

fn raw_write_all<S: OncRpc + ?Sized>(s: &mut S, buf: &[u8]) -> Result<()> {
    let mut buf = buf;
    while !buf.is_empty() {
        match s.raw_write(buf) {
            Ok(0) => {
                return Err(Error::new(
                    ErrorKind::WriteZero,
                    "failed to write whole buffer",
                ));
            }
            Ok(n) => buf = &buf[n..],
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn raw_read_exact<S: OncRpc + ?Sized>(s: &mut S, mut buf: &mut [u8]) -> Result<()> {
    while !buf.is_empty() {
        match s.raw_read(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    if !buf.is_empty() {
        Err(Error::new(
            ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ))
    } else {
        Ok(())
    }
}
