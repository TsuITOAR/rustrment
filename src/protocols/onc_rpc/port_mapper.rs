use super::xdr::*;
use super::{OncRpc, OncRpcBroadcast};
use bytes::{Buf, BytesMut};
use std::{
    io::{Read, Result, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket},
    time::Duration,
};

pub const PORT: u16 = 111;
pub enum IpProtocol {
    Tcp,
    Udp,
}
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
    pub fn new_tcp<A: ToSocketAddrs, D: Into<Option<Duration>>>(
        addr: A,
        dur: D,
    ) -> Result<PortMapper<TcpStream>> {
        let io = TcpStream::connect(addr)?;
        io.set_read_timeout(dur.into())?;
        Ok(PortMapper {
            io,
            buffer: BytesMut::new(),
        })
    }
    pub fn get_port(&mut self, prog: u32, vers: u32, ip_pro: IpProtocol) -> Result<u32> {
        let mut b: bytes::Bytes = self.call_anonymously(
            Procedure::GetPort,
            &mapping {
                port: 0,
                prog,
                prot: match ip_pro {
                    IpProtocol::Tcp => super::xdr::IPPROTO_TCP,
                    IpProtocol::Udp => IPPROTO_UDP,
                },
                vers,
            },
        )?;
        Ok(b.get_u32())
    }
    pub fn tcp_port(&mut self, prog: u32, vers: u32) -> Result<u32> {
        self.get_port(prog, vers, IpProtocol::Tcp)
    }
    pub fn udp_port(&mut self, prog: u32, vers: u32) -> Result<u32> {
        self.get_port(prog, vers, IpProtocol::Udp)
    }
}
impl PortMapper<UdpSocket> {
    pub fn new_udp<L: ToSocketAddrs, D: Into<Option<Duration>>>(
        addr: L,
        dur: D,
    ) -> Result<PortMapper<UdpSocket>> {
        let io = UdpSocket::bind(addr)?;
        io.set_read_timeout(dur.into())?;
        Ok(PortMapper {
            io,
            buffer: BytesMut::new(),
        })
    }
    pub fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> Result<()> {
        self.io.connect(addr)
    }
    pub fn get_port<A: ToSocketAddrs>(
        &mut self,
        prog: u32,
        vers: u32,
        ip_pro: IpProtocol,
        addr: A,
    ) -> Result<u32> {
        let mut b: bytes::Bytes = self.call_to_anonymously(
            Procedure::GetPort,
            &mapping {
                port: 0,
                prog,
                prot: match ip_pro {
                    IpProtocol::Tcp => super::xdr::IPPROTO_TCP,
                    IpProtocol::Udp => IPPROTO_UDP,
                },
                vers,
            },
            addr,
        )?;
        Ok(b.get_u32())
    }
    pub fn tcp_port<A: ToSocketAddrs>(&mut self, prog: u32, vers: u32, addr: A) -> Result<u32> {
        self.get_port(prog, vers, IpProtocol::Tcp, addr)
    }
    pub fn udp_port<A: ToSocketAddrs>(&mut self, prog: u32, vers: u32, addr: A) -> Result<u32> {
        self.get_port(prog, vers, IpProtocol::Udp, addr)
    }
    pub fn collect_port<'a, A: ToSocketAddrs>(
        &'a mut self,
        prog: u32,
        vers: u32,
        ip_pro: IpProtocol,
        addr: A,
    ) -> Result<impl Iterator<Item = Result<(u32, SocketAddr)>> + 'a> {
        let stream = self.broadcast_anonymously(
            Procedure::GetPort,
            &mapping {
                port: 0,
                prog,
                prot: match ip_pro {
                    IpProtocol::Tcp => IPPROTO_TCP,
                    IpProtocol::Udp => IPPROTO_UDP,
                },
                vers,
            },
            addr,
        )?;
        Ok(stream.map(
            |x: Result<(bytes::Bytes, SocketAddr)>| -> Result<(u32, SocketAddr)> {
                match x {
                    Ok((b, a)) => Ok((
                        serde_xdr::from_bytes::<_, u32>(b)
                            .map_err(|x| std::io::Error::new(std::io::ErrorKind::Other, x))?,
                        a,
                    )),
                    Err(e) => Err(e),
                }
            },
        ))
    }
    pub fn collet_tcp_port<'a, A: ToSocketAddrs + 'a>(
        &'a mut self,
        prog: u32,
        vers: u32,
        addr: A,
    ) -> Result<impl Iterator<Item = Result<(u32, SocketAddr)>> + 'a> {
        self.collect_port(prog, vers, IpProtocol::Tcp, addr)
    }
    pub fn collet_udp_port<'a, A: ToSocketAddrs + 'a>(
        &'a mut self,
        prog: u32,
        vers: u32,
        addr: A,
    ) -> Result<impl Iterator<Item = Result<(u32, SocketAddr)>> + 'a> {
        self.collect_port(prog, vers, IpProtocol::Udp, addr)
    }
}

impl OncRpc for PortMapper<TcpStream> {
    const PROGRAM: u32 = 100000;
    const VERSION: u32 = 2;
    fn raw_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.io.read(buf)
    }
    fn raw_write(&mut self, buf: &[u8]) -> Result<usize> {
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
    fn listen<A: std::net::ToSocketAddrs>(&mut self, addr: A) -> Result<()> {
        self.connect(addr)
    }
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
