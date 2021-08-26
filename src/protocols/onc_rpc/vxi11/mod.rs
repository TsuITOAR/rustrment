use std::{
    fmt::Debug,
    io::Result,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    time::Duration,
};

use bytes::{buf::Reader, Buf, Bytes};
use serde::Serialize;

use crate::protocols::onc_rpc::RpcProgram;

use self::{abort::Abort, core::Core, interrupt::Interrupt};

use super::{
    port_mapper::{self, PortMapper},
    xdr,
};
pub mod abort;
pub mod core;
pub mod interrupt;

const VERSION: u32 = 1;

fn error_to_i32(l: xdr::Device_ErrorCode) -> i32 {
    (l.0).0
}
pub enum ErrorCode {
    ///No error
    NoError,
    ///Syntax error
    SyntaxError,
    ///device not accessible
    NotAccessible,
    ///invalid link identifier
    InvalidIdentifier,
    ///parameter error
    ParameterError,
    ///channel not established
    NotEstablished,
    ///operation not supported
    NotSupported,
    ///out of resources
    OutOfResources,
    ///device locked by another link
    LockedByAnother,
    ///no lock held by this link
    NoLockHeld,
    ///I/O timeout
    IOTimeOut,
    ///I/O error
    IOError,
    ///Invalid address
    InvalidAddress,
    ///abort
    Abort,
    ///channel already established
    AlreadyEstablished,
    ///Unknown error code
    Unknown(i32),
}
impl ToString for ErrorCode {
    fn to_string(&self) -> String {
        use ErrorCode::*;
        match self {
            NoError => "no error",
            SyntaxError => "syntax error",
            NotAccessible => "device not accessible",
            InvalidIdentifier => "invalid link identifier",
            ParameterError => "parameter error",
            NotEstablished => "channel not established",
            NotSupported => "operation not supported",
            OutOfResources => "out of resources",
            LockedByAnother => "device locked by another link",
            NoLockHeld => "no lock held by this link",
            IOTimeOut => "I/O timeout",
            IOError => "I/O error",
            InvalidAddress => "invalid address",
            Abort => "abort",
            AlreadyEstablished => "channel already established",
            Unknown(s) => return format!("unknown error code: {}", s),
        }
        .into()
    }
}
impl From<xdr::Device_ErrorCode> for ErrorCode {
    fn from(e: xdr::Device_ErrorCode) -> Self {
        use ErrorCode::*;
        let i: i32 = error_to_i32(e);
        match i {
            0 => NoError,
            1 => SyntaxError,
            3 => NotAccessible,
            4 => InvalidIdentifier,
            5 => ParameterError,
            6 => NotEstablished,
            8 => NotSupported,
            9 => OutOfResources,
            11 => LockedByAnother,
            12 => NoLockHeld,
            15 => IOTimeOut,
            17 => IOError,
            21 => InvalidAddress,
            23 => Abort,
            29 => AlreadyEstablished,
            n => Unknown(n),
        }
    }
}

impl From<xdr::Device_Error> for ErrorCode {
    fn from(e: xdr::Device_Error) -> Self {
        Self::from(e.error)
    }
}
impl From<ErrorCode> for std::io::Result<()> {
    fn from(e: ErrorCode) -> Self {
        use std::io::{Error, ErrorKind};
        use ErrorCode::*;
        match e {
            NoError => Ok(()),
            other => Err(Error::new(ErrorKind::Other, other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceFlags(i32);
impl DeviceFlags {
    pub fn new_zero() -> Self {
        Self(0)
    }
    pub fn wait_lock(mut self) -> Self {
        self.0 |= 1 << 0;
        self
    }
    pub fn end(mut self) -> Self {
        self.0 |= 1 << 3;
        self
    }
    pub fn terminator_set(mut self) -> Self {
        self.0 |= 1 << 7;
        self
    }
}
impl From<i32> for DeviceFlags {
    fn from(n: i32) -> Self {
        Self(n)
    }
}
impl From<DeviceFlags> for i32 {
    fn from(d: DeviceFlags) -> Self {
        d.0
    }
}

impl From<DeviceFlags> for xdr::Device_Flags {
    fn from(f: DeviceFlags) -> Self {
        Self(xdr::long(f.into()))
    }
}

const REQ_SIZE: usize = 512;
const TERM: char = '\n';
//https://zone.ni.com/reference/en-XX/help/370131S-01/ni-visa/visaresourcesyntaxandexamples/
const INTERFACE_NAME: &str = "TCPIP";
pub struct Vxi11Cilent {
    pub client_id: i32,
    pub lock: bool,
    pub lock_timeout: Duration,
    pub io_timeout: Duration,
    pub req_size: u32,
    pub term: char,
}

impl Vxi11Cilent {
    pub fn new(
        client_id: i32,
        lock: bool,
        lock_timeout: Duration,
        io_timeout: Duration,
        req_size: u32,
        term: char,
    ) -> Self {
        Self {
            client_id,
            io_timeout,
            lock_timeout,
            lock,
            req_size,
            term,
        }
    }
}
impl Default for Vxi11Cilent {
    fn default() -> Self {
        Self {
            client_id: 20210826,
            io_timeout: Duration::from_millis(500),
            lock_timeout: Duration::from_millis(500),
            lock: true,
            req_size: 100 * 1024 * 1024, //100M
            term: '\n',
        }
    }
}

pub struct Vxi11 {
    link_id: i32,
    lock_timeout: Duration,
    io_timeout: Duration,
    max_recv_size: u32,
    req_size: usize,
    term: char,
    core: Core<TcpStream>,
    abort: Abort<TcpStream>,
    interrupt: Option<Interrupt<TcpStream>>,
    data_to_read: Option<Reader<Bytes>>,
}

impl Vxi11 {
    pub fn new<A: ToSocketAddrs>(
        addr: A,
        client_id: i32,
        lock: bool,
        lock_timeout: Duration,
        io_timeout: Duration,
    ) -> Result<Self> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .expect("invalid socket address");
        let name = INTERFACE_NAME.to_string();
        let mut core = Core::new_tcp::<SocketAddr>(addr)?;
        let (link_id, abort_port, max_recv_size) =
            core.create_link(client_id, lock, lock_timeout.as_millis() as u32, name)?;
        let abort_addr = SocketAddr::new(addr.ip(), abort_port as u16);
        let abort = Abort::new(TcpStream::connect(abort_addr)?);
        Ok(Self {
            abort,
            core,
            interrupt: None,
            io_timeout,
            lock_timeout,
            link_id,
            max_recv_size,
            req_size: REQ_SIZE,
            term: TERM,
            data_to_read: None,
        })
    }
    pub fn mut_core(&mut self) -> &mut Core<TcpStream> {
        &mut self.core
    }
    pub fn mut_abort(&mut self) -> &mut Abort<TcpStream> {
        &mut self.abort
    }
    pub fn mut_interrupt(&mut self) -> Option<&mut Interrupt<TcpStream>> {
        (&mut self.interrupt).as_mut()
    }
    pub fn set_term(&mut self, term: char) -> &mut Self {
        self.term = term;
        self
    }
    pub fn set_req_size(&mut self, req_size: usize) -> &mut Self {
        self.req_size = req_size;
        self
    }
    pub fn set_io_timeout(&mut self, dur: Duration) -> &mut Self {
        self.io_timeout = dur;
        self
    }
    pub fn set_lock_timeout(&mut self, dur: Duration) -> &mut Self {
        self.lock_timeout = dur;
        self
    }
    pub fn device_write<M: AsRef<[u8]> + Debug + Serialize>(
        &mut self,
        message: M,
    ) -> Result<usize> {
        debug_assert!(message.as_ref().len() <= self.max_recv_size as usize);
        self.core.device_write(
            self.link_id,
            DeviceFlags::new_zero(),
            self.lock_timeout.as_millis() as u32,
            self.io_timeout.as_millis() as u32,
            message,
        )
    }
    pub fn device_read(&mut self, req_size: usize, term: char) -> Result<Bytes> {
        self.core.device_read(
            self.link_id,
            DeviceFlags::new_zero(),
            self.lock_timeout.as_millis() as u32,
            self.io_timeout.as_millis() as u32,
            req_size,
            term,
        )
    }
    pub fn device_abort(&mut self) -> Result<()> {
        self.abort.device_abort(self.link_id)
    }
    pub fn establish_interrupt<A: ToSocketAddrs>(
        &mut self,
        addr: A,
        prog_num: u32,
        prog_ver: u32,
    ) -> Result<()> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .expect("invalid socket address");
        let listener = TcpListener::bind(addr)?;
        self.core
            .create_intr_chan(addr, prog_num, prog_ver, super::IpProtocol::Tcp)?;
        let (interrupt, addr) = listener.accept()?;
        debug_assert_eq!(addr, self.core.mut_io().peer_addr()?);
        self.interrupt = Some(Interrupt::new(prog_num, prog_ver, interrupt));
        Ok(())
    }
}
impl std::io::Read for Vxi11 {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut reader = match self.data_to_read.take() {
            None => self.device_read(self.req_size, self.term)?.reader(),
            Some(r) => r,
        };
        let len = reader.read(buf)?;
        if reader.get_ref().has_remaining() {
            self.data_to_read = Some(reader);
        }
        Ok(len)
    }
}

impl std::io::Write for Vxi11 {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.device_write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.mut_core().get_io().flush()
    }
}
impl crate::Protocol for Vxi11Cilent {
    type Address = IpAddr;
    type Error = std::io::Error;
    type IO = Vxi11;
    fn connect(self, address: Self::Address) -> std::result::Result<Self::IO, Self::Error> {
        let mut port_mapper =
            PortMapper::new_tcp(SocketAddr::new(address, port_mapper::PORT), self.io_timeout)?;
        let core_port = port_mapper.get_port(
            <Core<TcpStream> as RpcProgram>::PROGRAM,
            <Core<TcpStream> as RpcProgram>::VERSION,
            super::IpProtocol::Tcp,
        )?;
        let vxi11_addr = SocketAddr::new(address, core_port as u16);
        Vxi11::new(
            vxi11_addr,
            self.client_id,
            self.lock,
            self.lock_timeout,
            self.io_timeout,
        )
    }
}
