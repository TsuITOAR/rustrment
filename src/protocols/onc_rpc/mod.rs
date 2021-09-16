pub mod oncrpc_error;
pub mod port_mapper;
pub mod vxi11;
use crate::Result;
use bytes::{BufMut, Bytes, BytesMut};
use onc_rpc::{auth::AuthFlavor, CallBody, MessageType, ReplyBody, RpcMessage};
use serde::Serialize;
use std::{
    convert::{TryFrom, TryInto},
    net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket},
    time::Duration,
};
include!(concat!(env!("OUT_DIR"), r#"/xdr.rs"#));

pub enum IpProtocol {
    Tcp,
    Udp,
}

const HEAD_LEN: usize = 4;
fn parse_bytes(bytes: Bytes) -> Result<RpcMessage<Bytes, Bytes>> {
    match RpcMessage::try_from(bytes) {
        Ok(m) => Ok(m),
        Err(onc_rpc::Error::IncompleteHeader) | Err(onc_rpc::Error::IncompleteMessage { .. }) => {
            unreachable!()
        }
        Err(e) => Err(e.into()),
    }
}
fn expected_message_len(data: &[u8]) -> (usize, bool) {
    let header = u32::from_be_bytes(data.try_into().expect("header need at least 4 bytes"));
    ((header & (!(1 << 31))) as usize, (header & (1 << 31)) != 0)
}
pub trait RpcStream {
    fn raw_write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    fn raw_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    fn flush(&mut self) -> std::io::Result<()>;
    fn send<T, P>(&mut self, message: RpcMessage<T, P>) -> Result<()>
    where
        T: AsRef<[u8]>,
        P: AsRef<[u8]>,
    {
        raw_write_all(self, &message.serialise()?)?;
        self.flush()?;
        Ok(())
    }

    fn read(&mut self, buf: BytesMut) -> Result<RpcMessage<Bytes, Bytes>> {
        let mut head_buf = [0_u8; 4];
        let mut buf_cursor = MyCursor::new(buf);
        let mut total_len = 0;
        buf_cursor.reserve(HEAD_LEN);
        buf_cursor.advance(HEAD_LEN);
        raw_read_exact(self, head_buf.as_mut())?;
        loop {
            let (this_len, is_last) = expected_message_len(head_buf.as_ref());
            buf_cursor.reserve(this_len);
            raw_read_exact(self, &mut buf_cursor.as_mut()[..this_len as usize])?;
            buf_cursor.advance(this_len);
            total_len += this_len;
            if is_last {
                break;
            } else {
                raw_read_exact(self, head_buf.as_mut())?;
            }
        }
        //for now use the non-support-for-fragment onc-rpc crate, which can't handle head of bigger than 2^31-1, about 2GB
        //TODO: use own convert function to support message with any length
        assert!(total_len < (1 << 31));
        let fake_head = (total_len | (1 << 31)) as u32;
        let mut buf = buf_cursor.into_inner();
        buf.as_mut()[..HEAD_LEN].copy_from_slice(fake_head.to_be_bytes().as_ref());
        Ok(parse_bytes(buf.split_to(total_len + HEAD_LEN).freeze())?)
    }
    fn set_read_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()>;
    fn set_write_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()>;
}

const UDP_BUFFER_LEN: usize = 256;
pub trait RpcSocket {
    fn raw_send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> std::io::Result<usize>;
    fn raw_recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)>;
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
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    )
                    .into());
                }
                Ok(n) if n == buf.len() => return Ok(()),
                Ok(n) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "only {} byte(s) message sent, expected {} bytes",
                            n,
                            buf.len()
                        ),
                    )
                    .into())
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
    fn recv_from(&self, mut buf: BytesMut) -> Result<(RpcMessage<Bytes, Bytes>, SocketAddr)> {
        buf.reserve(UDP_BUFFER_LEN + HEAD_LEN); //reserve uninitialized length
        buf.put_slice(&mut [0_u8; HEAD_LEN][..]);
        let mut buf_cursor = MyCursor::new(buf);
        buf_cursor.advance(HEAD_LEN);
        buf_cursor.reserve(UDP_BUFFER_LEN); //reserve unfilled but initialized length
        let (bytes, addr) = {
            //UDP don't fragment
            let (num_read, addr) = self.raw_recv_from(buf_cursor.as_mut())?;
            let mut buf = buf_cursor.into_inner();
            let head: [u8; HEAD_LEN] = (num_read as u32).to_be_bytes();
            assert!((num_read as u32) < (u32::MAX >> 1));
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
    fn set_read_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()>;
    fn set_write_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()>;
}

fn raw_write_all<S: RpcStream + ?Sized>(s: &mut S, buf: &[u8]) -> Result<()> {
    let mut buf = buf;
    while !buf.is_empty() {
        match s.raw_write(buf) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "failed to write whole buffer",
                )
                .into());
            }
            Ok(n) => buf = &buf[n..],
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn raw_read_exact<S: RpcStream + ?Sized>(s: &mut S, mut buf: &mut [u8]) -> Result<()> {
    while !buf.is_empty() {
        match s.raw_read(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    if !buf.is_empty() {
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        )
        .into())
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
const REDUNDANCE: usize = 5;
impl MyCursor<BytesMut> {
    fn reserve(&mut self, additional: usize) {
        if self.filled + additional > self.capacity() {
            self.inner.resize(self.filled + additional * REDUNDANCE, 0);
        }
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> AsRef<[u8]> for MyCursor<T> {
    fn as_ref(&self) -> &[u8] {
        assert!(self.inner.as_ref().len() >= self.filled);
        &self.inner.as_ref()[..self.filled]
    }
}

impl<T: AsRef<[u8]> + AsMut<[u8]>> AsMut<[u8]> for MyCursor<T> {
    fn as_mut(&mut self) -> &mut [u8] {
        assert!(self.inner.as_ref().len() >= self.filled);
        &mut self.inner.as_mut()[self.filled..]
    }
}

fn stream_receive<S, R>(s: &S, xid: u32) -> Result<(R, SocketAddr)>
where
    S: RpcBroadcast + RpcProgram + ?Sized,
    <S as RpcProgram>::IO: RpcSocket,
    R: TryFrom<Bytes>,
    crate::error::Error: From<<R as TryFrom<bytes::Bytes>>::Error>,
{
    {
        let buf = s.buffer();
        let (reply, addr) = match s.get_io().recv_from(buf)? {
            (r, addr) => (r, addr),
        };
        if reply.xid() == xid {
            match reply.reply_body().ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "expected reply, found call",
            ))? {
                ReplyBody::Accepted(a) => match a.status() {
                    onc_rpc::AcceptedStatus::Success(p) => Ok((p.clone().try_into()?, addr)),

                    u => Err(oncrpc_error::UnsuccessfulAcceptStatus::from(
                        oncrpc_error::PrivateWrapper(u),
                    )
                    .into()),
                },
                ReplyBody::Denied(d) => Err(oncrpc_error::RejectedReply::from(d).into()),
            }
        } else {
            Err(oncrpc_error::OncRpcError::XidUnmatched(xid, reply.xid()).into())
        }
    }
}

pub trait RpcProgram {
    type IO;
    const PROGRAM: u32;
    const VERSION: u32;
    fn gen_xid(&mut self) -> u32 {
        rand::random()
    }
    fn get_io(&self) -> &Self::IO;
    fn mut_io(&mut self) -> &mut Self::IO;
    fn buffer(&self) -> BytesMut;
}

pub trait Rpc {
    fn call<P, T, C, R>(
        &mut self,
        procedure: P,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: C,
    ) -> Result<R>
    where
        P: Into<u32>,
        T: AsRef<[u8]>,
        C: Serialize,
        R: TryFrom<Bytes>,
        crate::error::Error: From<<R as TryFrom<bytes::Bytes>>::Error>;
    fn call_anonymously<P, C, R>(&mut self, procedure: P, content: C) -> Result<R>
    where
        P: Into<u32>,
        C: Serialize,
        R: TryFrom<Bytes>,
        crate::error::Error: From<<R as TryFrom<bytes::Bytes>>::Error>,
    {
        self.call::<P, &[u8], C, R>(
            procedure,
            AuthFlavor::AuthNone(None),
            AuthFlavor::AuthNone(None),
            content,
        )
    }
}

pub trait RpcBroadcast {
    fn broadcast<'a, P, T, C, A, R>(
        &'a mut self,
        procedure: P,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: C,
        addr: A,
    ) -> Result<std::iter::FromFn<Box<dyn FnMut() -> Option<Result<(R, SocketAddr)>> + 'a>>>
    where
        P: Into<u32>,
        T: AsRef<[u8]>,
        C: Serialize,
        A: ToSocketAddrs,
        R: TryFrom<Bytes>,
        crate::error::Error: From<<R as TryFrom<bytes::Bytes>>::Error>;
    fn broadcast_anonymously<'a, P, C, A, R>(
        &'a mut self,
        procedure: P,
        content: C,
        addr: A,
    ) -> Result<std::iter::FromFn<Box<dyn FnMut() -> Option<Result<(R, SocketAddr)>> + 'a>>>
    where
        P: Into<u32>,
        C: Serialize,
        A: ToSocketAddrs,
        R: TryFrom<Bytes>,
        crate::error::Error: From<<R as TryFrom<bytes::Bytes>>::Error>,
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

impl<S> Rpc for S
where
    S: RpcProgram,
    <S as RpcProgram>::IO: RpcStream,
{
    fn call<P, T, C, R>(
        &mut self,
        procedure: P,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: C,
    ) -> Result<R>
    where
        P: Into<u32>,
        T: AsRef<[u8]>,
        C: Serialize,
        R: TryFrom<Bytes>,
        crate::error::Error: From<<R as TryFrom<bytes::Bytes>>::Error>,
    {
        let xid = self.gen_xid();
        let content = serde_xdr::to_bytes(&content)?;
        let call_body = CallBody::new(
            Self::PROGRAM,
            Self::VERSION,
            procedure.into(),
            auth_credentials,
            auth_verifier,
            &content[..],
        );
        self.mut_io()
            .send(RpcMessage::new(xid, MessageType::Call(call_body)))?;
        let buf = self.buffer();
        let reply = self.mut_io().read(buf)?;
        if reply.xid() == xid {
            match reply.reply_body().ok_or(oncrpc_error::OncRpcError::Other(
                "expected reply, found call".to_string(),
            ))? {
                ReplyBody::Accepted(a) => match a.status() {
                    onc_rpc::AcceptedStatus::Success(p) => Ok(p.clone().try_into()?),

                    u => Err(oncrpc_error::UnsuccessfulAcceptStatus::from(
                        oncrpc_error::PrivateWrapper(u),
                    )
                    .into()),
                },
                ReplyBody::Denied(d) => Err(oncrpc_error::RejectedReply::from(d).into()),
            }
        } else {
            Err(oncrpc_error::OncRpcError::XidUnmatched(xid, reply.xid()).into())
        }
    }
}
impl<S> RpcBroadcast for S
where
    S: RpcProgram,
    <S as RpcProgram>::IO: RpcSocket,
{
    fn broadcast<'a, P, T, C, A, R>(
        &'a mut self,
        procedure: P,
        auth_credentials: AuthFlavor<T>,
        auth_verifier: AuthFlavor<T>,
        content: C,
        addr: A,
    ) -> Result<std::iter::FromFn<Box<dyn FnMut() -> Option<Result<(R, SocketAddr)>> + 'a>>>
    where
        P: Into<u32>,
        T: AsRef<[u8]>,
        C: Serialize,
        A: ToSocketAddrs,
        R: TryFrom<Bytes>,
        crate::error::Error: From<<R as TryFrom<bytes::Bytes>>::Error>,
    {
        let xid = self.gen_xid();
        let addr = addr
            .to_socket_addrs()?
            .next()
            .expect("invalid socket address");
        let content = serde_xdr::to_bytes(&content)?;
        let call_body = CallBody::new(
            Self::PROGRAM,
            Self::VERSION,
            procedure.into(),
            auth_credentials,
            auth_verifier,
            &content[..],
        );
        self.get_io()
            .send_to(RpcMessage::new(xid, MessageType::Call(call_body)), addr)?;
        Ok(std::iter::from_fn(Box::new(move || {
            Some(stream_receive(self, xid))
        })))
    }
}

impl RpcStream for TcpStream {
    fn raw_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(<Self as std::io::Read>::read(self, buf)?)
    }
    fn raw_write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(<Self as std::io::Write>::write(self, buf)?)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(<Self as std::io::Write>::flush(self)?)
    }
    fn set_read_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()> {
        Ok(TcpStream::set_read_timeout(self, dur.into())?)
    }
    fn set_write_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()> {
        Ok(TcpStream::set_write_timeout(self, dur.into())?)
    }
}
impl RpcStream for UdpSocket {
    fn raw_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(UdpSocket::recv(self, buf)?)
    }
    fn raw_write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(UdpSocket::send(self, buf)?)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
    fn send<T: AsRef<[u8]>, P: AsRef<[u8]>>(&mut self, message: RpcMessage<T, P>) -> Result<()> {
        Ok(send_without_head(self, message)?)
    }
    fn read(&mut self, buf: BytesMut) -> Result<RpcMessage<Bytes, Bytes>> {
        Ok(recv_without_head(self, buf)?)
    }
    fn set_read_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()> {
        Ok(UdpSocket::set_read_timeout(self, dur.into())?)
    }
    fn set_write_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()> {
        Ok(UdpSocket::set_write_timeout(self, dur.into())?)
    }
}

impl RpcSocket for UdpSocket {
    fn raw_recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        Ok(UdpSocket::recv_from(self, buf)?)
    }
    fn raw_send_to<A: ToSocketAddrs>(&self, buf: &[u8], addr: A) -> std::io::Result<usize> {
        Ok(UdpSocket::send_to(self, buf, addr)?)
    }
    fn set_read_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()> {
        Ok(UdpSocket::set_read_timeout(self, dur.into())?)
    }
    fn set_write_timeout<T: Into<Option<Duration>>>(&self, dur: T) -> Result<()> {
        Ok(UdpSocket::set_write_timeout(self, dur.into())?)
    }
}

fn send_without_head<S: RpcStream, T: AsRef<[u8]>, P: AsRef<[u8]>>(
    s: &mut S,
    message: RpcMessage<T, P>,
) -> Result<()> {
    let buf = message.serialise()?;
    let buf = &buf[HEAD_LEN..];
    if !buf.is_empty() {
        match s.raw_write(&buf) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "failed to write whole buffer",
                )
                .into());
            }
            Ok(n) if n == buf.len() => return Ok(()),
            Ok(n) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "only {} byte(s) message sent, expected {} bytes",
                        n,
                        buf.len()
                    ),
                )
                .into())
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn recv_without_head<S: RpcStream>(
    s: &mut S,
    mut buf: BytesMut,
) -> Result<RpcMessage<Bytes, Bytes>> {
    buf.reserve(UDP_BUFFER_LEN + HEAD_LEN); //reserve uninitialized length
    buf.put_slice(&mut [0_u8; HEAD_LEN][..]);
    let mut buf_cursor = MyCursor::new(buf);
    buf_cursor.advance(HEAD_LEN);
    buf_cursor.reserve(UDP_BUFFER_LEN); //reserve unfilled but initialized length
    let bytes = {
        //UDP don't fragment
        let num_read = s.raw_read(buf_cursor.as_mut())?;
        let mut buf = buf_cursor.into_inner();
        let head: [u8; HEAD_LEN] = (num_read as u32).to_be_bytes();
        assert!((num_read as u32) < (u32::MAX >> 1));
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
