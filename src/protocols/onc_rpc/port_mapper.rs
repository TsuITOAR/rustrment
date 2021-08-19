use super::{OncRpc, OncRpcBroadcast};
use bytes::BytesMut;
use std::{
    io::{Read, Result, Write},
    net::{IpAddr, SocketAddr, TcpStream, UdpSocket},
};

pub const PORT: u16 = 111;

pub struct PortMapper<S> {
    io: S,
    buffer: BytesMut,
}

impl<S> PortMapper<S> {
    pub fn new(io: S) -> Self {
        Self {
            io,
            buffer: BytesMut::new(),
        }
    }
    pub fn get_io(&self) -> &S {
        &self.io
    }
    pub fn mut_io(&mut self) -> &mut S {
        &mut self.io
    }
}
impl PortMapper<TcpStream> {
    pub fn new_tcp<A: Into<IpAddr>>(addr: A) -> Result<PortMapper<TcpStream>> {
        Ok(PortMapper {
            io: TcpStream::connect(SocketAddr::new(addr.into(), PORT))?,
            buffer: BytesMut::new(),
        })
    }
}
impl PortMapper<UdpSocket> {
    pub fn new_udp(local_port: u16) -> Result<PortMapper<UdpSocket>> {
        Ok(PortMapper {
            io: UdpSocket::bind(SocketAddr::new("127.0.0.1".parse().unwrap(), local_port))?,
            buffer: BytesMut::new(),
        })
    }
}

impl OncRpc for PortMapper<TcpStream> {
    const PROGRAM: u32 = 100000;
    const VERSION: u32 = 2;
    fn raw_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.io.read(buf)
    }
    fn raw_write(&mut self, buf: &[u8]) -> Result<usize> {
        println!("{}\n{:X?}", buf.len(), buf);
        self.io.write(buf)
    }
    fn buffer(&self) -> BytesMut {
        self.buffer.clone()
    }
    fn flush(&mut self) -> Result<()> {
        self.io.flush()
    }
}

impl OncRpcBroadcast for PortMapper<UdpSocket> {
    const PROGRAM: u32 = 100000;
    const VERSION: u32 = 2;
    fn buffer(&self) -> BytesMut {
        self.buffer.clone()
    }
    fn raw_recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.io.recv_from(buf)
    }
    fn raw_send_to<A: std::net::ToSocketAddrs>(&self, buf: &[u8], addr: A) -> Result<usize> {
        self.io.send_to(buf, addr)
    }
}

pub enum Procedure {
    Set,
    Unset,
    GetPort,
    //pamamplist unsupported yet
    //Dump,
    CallIt,
}
impl Into<u32> for Procedure {
    fn into(self) -> u32 {
        use Procedure::*;
        match self {
            Set => 1,
            Unset => 2,
            GetPort => 3,
            //Dump=>4,
            CallIt => 5,
        }
    }
}
