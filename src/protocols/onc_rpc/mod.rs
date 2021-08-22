pub mod port_mapper;
pub mod vxi11;
use bytes::{BufMut, Bytes, BytesMut};
use onc_rpc::{auth::AuthFlavor, CallBody, MessageType, ReplyBody, RpcMessage};
use serde::Serialize;
use std::{
    convert::{TryFrom, TryInto},
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
        let mut buf_cursor = MyCursor::new(self.buffer());
        buf_cursor.reserve(HEAD_LEN);
        let bytes = {
            let expected_len = loop {
                let num_read = self.raw_read(buf_cursor.as_mut())?;
                buf_cursor.advance(num_read);
                match onc_rpc::expected_message_len(buf_cursor.as_ref()) {
                    Ok(len) => break len as usize,
                    Err(onc_rpc::Error::IncompleteHeader) => continue,
                    Err(e) => return Err(Error::new(ErrorKind::InvalidData, e)),
                };
            };
            if expected_len > buf_cursor.filled {
                let current_len = buf_cursor.filled;
                buf_cursor.reserve(expected_len - current_len);

                // The buffer does not contain a full message, read more data
                raw_read_exact(self, &mut buf_cursor.as_mut()[..expected_len - current_len])?;
            }
            let mut buf = buf_cursor.into_inner();
            // Split the buffer into a single message
            let msg_bytes = buf.split_to(expected_len as usize);
            Result::Ok(msg_bytes.freeze())
        }?;
        Ok(parse_bytes(bytes)?)
    }
    fn call<P, T, C, R>(
        &mut self,
        procedure: P,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: &C,
    ) -> Result<R>
    where
        P: Into<u32>,
        T: AsRef<[u8]>,
        C: Serialize,
        R: TryFrom<Bytes>,
        <R as TryFrom<bytes::Bytes>>::Error: std::fmt::Display,
    {
        let xid = self.gen_xid();
        let content = serde_xdr::to_bytes(content).map_err(|x| Error::new(ErrorKind::Other, x))?;
        let call_body = CallBody::new(
            Self::PROGRAM,
            Self::VERSION,
            procedure.into(),
            auth_credentials,
            auth_verifier,
            &content[..],
        );
        self.send(RpcMessage::new(xid, MessageType::Call(call_body)))?;
        let reply = self.read()?;
        if reply.xid() == xid {
            match reply.reply_body().ok_or(Error::new(
                ErrorKind::InvalidInput,
                "expected reply, found call",
            ))? {
                ReplyBody::Accepted(a) => match a.status() {
                    onc_rpc::AcceptedStatus::Success(p) => {
                        (p.clone()
                            .try_into()
                            .map_err(|err: <R as TryFrom<Bytes>>::Error| {
                                Error::new(
                                    ErrorKind::Other,
                                    format!("err parsing data: {}", err.to_string()),
                                )
                            }))
                        .map_err(|e| Error::new(ErrorKind::Other, e))
                    }

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
    fn call_anonymously<P, C, R>(&mut self, procedure: P, content: &C) -> Result<R>
    where
        P: Into<u32>,
        C: Serialize,
        R: TryFrom<Bytes>,
        <R as TryFrom<bytes::Bytes>>::Error: std::fmt::Display,
    {
        self.call::<P, &[u8], C, R>(
            procedure,
            AuthFlavor::AuthNone(None),
            AuthFlavor::AuthNone(None),
            content,
        )
    }
}

const UDP_BUFFER_LEN: usize = 256;
pub trait OncRpcBroadcast {
    const PROGRAM: u32;
    const VERSION: u32;

    fn raw_send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> Result<usize>;
    fn raw_recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>;
    fn raw_recv(&self, buf: &mut [u8]) -> Result<usize>;
    fn buffer(&self) -> BytesMut;
    fn listen<A: ToSocketAddrs>(&mut self, addr: A) -> Result<()>;
    fn gen_xid(&mut self) -> u32 {
        rand::random()
    }
    fn recv(&mut self) -> Result<RpcMessage<Bytes, Bytes>> {
        let mut buf = self.buffer();
        buf.reserve(UDP_BUFFER_LEN + HEAD_LEN);
        buf.put_slice(&mut [0_u8; HEAD_LEN][..]);
        let mut buf_cursor = MyCursor::new(self.buffer());
        buf_cursor.advance(HEAD_LEN);
        buf_cursor.reserve(UDP_BUFFER_LEN);
        let bytes = {
            //UDP don't fragment
            let num_read = self.raw_recv(buf_cursor.as_mut())?;
            let mut buf = buf_cursor.into_inner();
            let head: [u8; HEAD_LEN] = (num_read as u32).to_be_bytes();
            debug_assert!((num_read as u32) < (u32::MAX >> 1));
            for i in 0..HEAD_LEN {
                buf[i] = head[i];
            }
            buf[0] |= 0b10000000;
            // Split the buffer into a single message
            let msg_bytes = buf.split_to(num_read + HEAD_LEN as usize);
            msg_bytes.freeze()
        };
        Ok(parse_bytes(bytes)?)
    }
    fn recv_from(&mut self) -> Result<(RpcMessage<Bytes, Bytes>, SocketAddr)> {
        let mut buf = self.buffer();
        buf.reserve(UDP_BUFFER_LEN + HEAD_LEN);
        buf.put_slice(&mut [0_u8; HEAD_LEN][..]);
        let mut buf_cursor = MyCursor::new(self.buffer());
        buf_cursor.advance(HEAD_LEN);
        buf_cursor.reserve(UDP_BUFFER_LEN);
        let (bytes, addr) = {
            //UDP don't fragment
            let (num_read, addr) = self.raw_recv_from(buf_cursor.as_mut())?;
            let mut buf = buf_cursor.into_inner();
            let head: [u8; HEAD_LEN] = (num_read as u32).to_be_bytes();
            debug_assert!((num_read as u32) < (u32::MAX >> 1));
            for i in 0..HEAD_LEN {
                buf[i] = head[i];
            }
            buf[0] |= 0b10000000;
            // Split the buffer into a single message
            let msg_bytes = buf.split_to(num_read + HEAD_LEN as usize);
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
        let buf = &buf[HEAD_LEN..];
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
    fn call_to<P, T, C, A, R>(
        &mut self,
        procedure: P,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: &C,
        addr: A,
    ) -> Result<R>
    where
        P: Into<u32>,
        T: AsRef<[u8]>,
        C: Serialize,
        A: ToSocketAddrs,
        R: TryFrom<Bytes>,
        <R as TryFrom<bytes::Bytes>>::Error: std::fmt::Display,
    {
        let xid = self.gen_xid();
        let addr = addr
            .to_socket_addrs()?
            .next()
            .expect("invalid socket address");
        let content = serde_xdr::to_bytes(content).map_err(|x| Error::new(ErrorKind::Other, x))?;
        let call_body = CallBody::new(
            Self::PROGRAM,
            Self::VERSION,
            procedure.into(),
            auth_credentials,
            auth_verifier,
            &content[..],
        );
        self.listen(addr)?;
        self.send_to(RpcMessage::new(xid, MessageType::Call(call_body)), addr)?;
        let reply = self.recv()?;
        if reply.xid() == xid {
            match reply.reply_body().ok_or(Error::new(
                ErrorKind::InvalidInput,
                "expected reply, found call",
            ))? {
                ReplyBody::Accepted(a) => match a.status() {
                    onc_rpc::AcceptedStatus::Success(p) => {
                        p.clone()
                            .try_into()
                            .map_err(|err: <R as TryFrom<Bytes>>::Error| {
                                Error::new(
                                    ErrorKind::Other,
                                    format!("err parsing data: {}", err.to_string()),
                                )
                            })
                    }

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
    fn call_to_anonymously<P, C, A, R>(&mut self, procedure: P, content: &C, addr: A) -> Result<R>
    where
        P: Into<u32>,
        C: Serialize,
        A: ToSocketAddrs,
        R: TryFrom<Bytes>,
        <R as TryFrom<bytes::Bytes>>::Error: std::fmt::Display,
    {
        self.call_to::<P, &[u8], C, A, R>(
            procedure,
            AuthFlavor::AuthNone(None),
            AuthFlavor::AuthNone(None),
            content,
            addr,
        )
    }
    fn broadcast<'a, P, T, C, A, R>(
        &'a mut self,
        procedure: P,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: &C,
        addr: A,
    ) -> Result<std::iter::FromFn<Box<dyn FnMut() -> Option<Result<(R, SocketAddr)>> + 'a>>>
    where
        P: Into<u32>,
        T: AsRef<[u8]>,
        C: Serialize,
        A: ToSocketAddrs,
        R: TryFrom<Bytes>,
        <R as TryFrom<bytes::Bytes>>::Error: std::fmt::Display,
    {
        let xid = self.gen_xid();
        let addr = addr
            .to_socket_addrs()?
            .next()
            .expect("invalid socket address");
        let content = serde_xdr::to_bytes(content).map_err(|x| Error::new(ErrorKind::Other, x))?;
        let call_body = CallBody::new(
            Self::PROGRAM,
            Self::VERSION,
            procedure.into(),
            auth_credentials,
            auth_verifier,
            &content[..],
        );
        self.send_to(RpcMessage::new(xid, MessageType::Call(call_body)), addr)?;
        Ok(std::iter::from_fn(Box::new(move || {
            Some(stream_receive(self, xid))
        })))
    }
    fn broadcast_anonymously<'a, P, C, A, R>(
        &'a mut self,
        procedure: P,
        content: &C,
        addr: A,
    ) -> Result<std::iter::FromFn<Box<dyn FnMut() -> Option<Result<(R, SocketAddr)>> + 'a>>>
    where
        P: Into<u32>,
        C: Serialize,
        A: ToSocketAddrs,
        R: TryFrom<Bytes>,
        <R as TryFrom<bytes::Bytes>>::Error: std::fmt::Display,
    {
        self.broadcast::<P, &[u8], C, A, R>(
            procedure,
            AuthFlavor::AuthNone(None),
            AuthFlavor::AuthNone(None),
            content,
            addr,
        )
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

struct MyCursor<T: AsRef<[u8]> + AsMut<[u8]>> {
    inner: T,
    filled: usize,
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> MyCursor<T> {
    fn new(inner: T) -> Self {
        Self { inner, filled: 0 }
    }
    fn advance(&mut self, step: usize) -> &mut Self {
        self.filled += step;
        self
    }
    fn into_inner(self) -> T {
        self.inner
    }
    fn capacity(&self) -> usize {
        self.inner.as_ref().len()
    }
}
impl MyCursor<BytesMut> {
    fn reserve(&mut self, additional: usize) {
        if self.filled + additional > self.capacity() {
            self.inner.resize(self.filled + additional, 0);
        }
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> AsRef<[u8]> for MyCursor<T> {
    fn as_ref(&self) -> &[u8] {
        debug_assert!(self.inner.as_ref().len() >= self.filled);
        &self.inner.as_ref()[..self.filled]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> AsMut<[u8]> for MyCursor<T> {
    fn as_mut(&mut self) -> &mut [u8] {
        debug_assert!(self.inner.as_ref().len() >= self.filled);
        &mut self.inner.as_mut()[self.filled..]
    }
}

fn stream_receive<S: OncRpcBroadcast + ?Sized, R>(s: &mut S, xid: u32) -> Result<(R, SocketAddr)>
where
    S: OncRpcBroadcast,
    R: TryFrom<Bytes>,
    <R as TryFrom<bytes::Bytes>>::Error: std::fmt::Display,
{
    {
        let (reply, addr) = match s.recv_from()? {
            (r, addr) => (r, addr),
        };
        if reply.xid() == xid {
            match reply.reply_body().ok_or(Error::new(
                ErrorKind::InvalidInput,
                "expected reply, found call",
            ))? {
                ReplyBody::Accepted(a) => match a.status() {
                    onc_rpc::AcceptedStatus::Success(p) => Ok((
                        p.clone()
                            .try_into()
                            .map_err(|err: <R as TryFrom<Bytes>>::Error| {
                                Error::new(
                                    ErrorKind::Other,
                                    format!("err parsing data: {}", err.to_string()),
                                )
                            })?,
                        addr,
                    )),

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
}
