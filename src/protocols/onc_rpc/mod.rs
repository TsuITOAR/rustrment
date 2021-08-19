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
    fn raw_write(&mut self, buf: &[u8]) -> Result<usize>;
    fn raw_read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn buffer(&self) -> BytesMut;
    fn flush(&mut self) -> Result<()>;
    fn gen_xid(&mut self) -> u32 {
        rand::random()
    }

    fn send<T, P>(&mut self, message: RpcMessage<T, P>) -> Result<()>
    where
        T: AsRef<[u8]>,
        P: AsRef<[u8]>,
    {
        raw_write_all(self, &message.serialise()?)?;
        self.flush()
    }

    fn read(&mut self) -> Result<RpcMessage<Bytes, Bytes>> {
        let mut buf = self.buffer().clone();
        if buf.len() < HEAD_LEN {
            buf.resize(HEAD_LEN, 0);
        };
        let bytes = {
            let mut buf_cursor = std::io::Cursor::new(buf);
            let expected_len = loop {
                let current_pos = buf_cursor.position() as usize;
                let num_read = self.raw_read(&mut buf_cursor.get_mut()[current_pos..])? as u64;
                buf_cursor.set_position(buf_cursor.position() + num_read);
                match onc_rpc::expected_message_len(
                    &buf_cursor.get_ref()[..buf_cursor.position() as usize],
                ) {
                    Ok(len) => break len as usize,
                    Err(onc_rpc::Error::IncompleteHeader) => continue,
                    Err(e) => return Err(Error::new(ErrorKind::InvalidData, e)),
                };
            };
            let current_pos = buf_cursor.position() as usize;
            buf = buf_cursor.into_inner();
            if expected_len > current_pos {
                let temp = buf.len();
                if expected_len > temp {
                    buf.resize(expected_len, 0);
                }

                // The buffer does not contain a full message, read more data
                raw_read_exact(self, &mut buf[current_pos..expected_len])?;
            }

            // Split the buffer into a single message
            let msg_bytes = buf.split_to(expected_len as usize);
            Result::Ok(msg_bytes.freeze())
        }?;
        Ok(parse_bytes(bytes)?)
    }
    fn call<P: Into<u32>, T: AsRef<[u8]>, C: AsRef<[u8]>>(
        &mut self,
        procedure: P,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: C,
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
        self.send(RpcMessage::new(xid, MessageType::Call(call_body)))?;
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
    fn call_anonymously<P: Into<u32>, C: AsRef<[u8]>>(
        &mut self,
        procedure: P,
        content: C,
    ) -> Result<Bytes> {
        self.call::<P, &[u8], C>(
            procedure,
            AuthFlavor::AuthNone(None),
            AuthFlavor::AuthNone(None),
            content,
        )
    }
}

pub trait OncRpcBroadcast {
    const PROGRAM: u32;
    const VERSION: u32;

    fn raw_send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> Result<usize>;
    fn raw_recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>;
    fn buffer(&self) -> BytesMut;
    fn recv_from(&mut self) -> Result<(RpcMessage<Bytes, Bytes>, SocketAddr)> {
        let mut buf = self.buffer().clone();
        if buf.len() < HEAD_LEN {
            buf.resize(HEAD_LEN, 0);
        };

        let (bytes, addr) = {
            let mut buf_cursor = std::io::Cursor::new(buf);
            //UDP don't fragment
            let current_pos = buf_cursor.position() as usize;
            let (num_read, addr) =
                self.raw_recv_from(&mut buf_cursor.get_mut()[current_pos..])? as (usize, _);
            buf_cursor.set_position(buf_cursor.position() + num_read as u64);

            let expected_len = match onc_rpc::expected_message_len(
                &buf_cursor.get_ref()[..buf_cursor.position() as usize], /* zero-copy clone */
            ) {
                Ok(len) => len as usize,
                Err(e) => return Err(Error::new(ErrorKind::InvalidData, e)),
            };
            let current_pos = buf_cursor.position() as usize;
            buf = buf_cursor.into_inner();
            if expected_len > current_pos {
                let temp = buf.len();
                if expected_len > temp {
                    buf.resize(expected_len, 0);
                }
                let mut buf = &mut buf[current_pos..expected_len];
                // The buffer does not contain a full message, read more data
                while !buf.is_empty() {
                    match self.raw_recv_from(buf) {
                        Ok((0, a)) if a == addr => break,
                        Ok((n, a)) if a == addr => {
                            let tmp = buf;
                            buf = &mut tmp[n..];
                        }
                        Ok(_) => {
                            unimplemented!()
                        }
                        Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                        Err(e) => return Err(e),
                    }
                }
                if !buf.is_empty() {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof,
                        "failed to fill whole buffer",
                    ));
                }
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
