use std::{
    fmt::Debug,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    time::Duration,
};

use bytes::Bytes;

use crate::protocols::onc_rpc::RpcProgram;

use self::{abort::Abort, core::Core, interrupt::Interrupt};

use super::{
    port_mapper::{self, PortMapper},
    xdr,
};
pub mod abort;
pub mod core;
pub mod interrupt;
pub mod vxi11_error;
use crate::Result;
const VERSION: u32 = 1;

fn error_to_i32(l: xdr::Device_ErrorCode) -> i32 {
    (l.0).0
}
pub enum ErrorCode {
    ///No error
    NoError,
    ///syntax error
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
    ///unknown error code
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
impl From<ErrorCode> for Result<()> {
    fn from(e: ErrorCode) -> Self {
        use ErrorCode::*;

        Err(match e {
            NoError => return Ok(()),
            SyntaxError => vxi11_error::Vxi11Error::SyntaxError,
            NotAccessible => vxi11_error::Vxi11Error::NotAccessible,
            InvalidIdentifier => vxi11_error::Vxi11Error::InvalidIdentifier,
            ParameterError => vxi11_error::Vxi11Error::ParameterError,
            NotEstablished => vxi11_error::Vxi11Error::NotEstablished,
            NotSupported => vxi11_error::Vxi11Error::NotSupported,
            OutOfResources => vxi11_error::Vxi11Error::OutOfResources,
            LockedByAnother => vxi11_error::Vxi11Error::LockedByAnother,
            NoLockHeld => vxi11_error::Vxi11Error::NoLockHeld,
            IOTimeOut => vxi11_error::Vxi11Error::IOTimeOut,
            IOError => vxi11_error::Vxi11Error::IOError,
            InvalidAddress => vxi11_error::Vxi11Error::InvalidAddress,
            Abort => vxi11_error::Vxi11Error::Abort,
            AlreadyEstablished => vxi11_error::Vxi11Error::AlreadyEstablished,
            Unknown(n) => vxi11_error::Vxi11Error::Vxi11Unknown(n),
        }
        .into())
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
//matlab instrument control box send inst0
const INTERFACE_NAME: &str = "inst0";
pub struct Vxi11Client {
    pub client_id: i32,
    pub lock: bool,
    pub lock_timeout: Duration,
    pub io_timeout: Duration,
    pub req_size: u32,
    pub term: char,
    pub flags: DeviceFlags,
}

impl Vxi11Client {
    pub fn new(
        client_id: i32,
        lock: bool,
        lock_timeout: Duration,
        io_timeout: Duration,
        req_size: u32,
        term: char,
        flags: DeviceFlags,
    ) -> Self {
        Self {
            client_id,
            io_timeout,
            lock_timeout,
            lock,
            req_size,
            term,
            flags,
        }
    }
}
impl Default for Vxi11Client {
    fn default() -> Self {
        Self {
            client_id: 20210826,
            io_timeout: Duration::from_millis(500),
            lock_timeout: Duration::from_millis(500),
            lock: true,
            req_size: 100 * 1024 * 1024, //100M
            term: '\n',
            flags: DeviceFlags::new_zero().terminator_set(),
        }
    }
}

pub struct Vxi11 {
    link_id: i32,
    lock_timeout: u32,
    io_timeout: u32,
    max_recv_size: u32,
    req_size: usize,
    term: char,
    flags: DeviceFlags,
    core: Core<TcpStream>,
    abort: Option<Abort<TcpStream>>,
    interrupt: Option<Interrupt<TcpStream>>,
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
        let abort = match TcpStream::connect(abort_addr) {
            Ok(c) => Some(Abort::new(c)),
            Err(e) => {
                //TO-DO there should be a warning message
                println!(
                    "failed establishing abort channel on '{}': {}",
                    abort_addr.to_string(),
                    e.to_string()
                );
                None
            }
        };
        Ok(Self {
            abort,
            core,
            interrupt: None,
            io_timeout: io_timeout.as_millis() as u32,
            lock_timeout: lock_timeout.as_millis() as u32,
            link_id,
            max_recv_size,
            req_size: REQ_SIZE,
            term: TERM,
            flags: DeviceFlags::new_zero().terminator_set(),
        })
    }
    pub fn mut_core(&mut self) -> &mut Core<TcpStream> {
        &mut self.core
    }
    pub fn mut_abort(&mut self) -> &mut Option<Abort<TcpStream>> {
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
        self.io_timeout = dur.as_millis() as u32;
        self
    }
    pub fn set_lock_timeout(&mut self, dur: Duration) -> &mut Self {
        self.lock_timeout = dur.as_millis() as u32;
        self
    }
    pub fn set_flags(&mut self, flags: DeviceFlags) -> &mut Self {
        self.flags = flags;
        self
    }
    pub fn device_write<M: AsRef<[u8]>>(&mut self, message: M) -> Result<usize> {
        debug_assert!(message.as_ref().len() <= self.max_recv_size as usize);
        self.core.device_write(
            self.link_id,
            self.flags,
            self.lock_timeout,
            self.io_timeout,
            message,
        )
    }
    pub fn device_write_str<S: AsRef<str>>(&mut self, message: S) -> Result<usize> {
        let message = message.as_ref().as_bytes();
        let mut temp;
        let mess = if message.last().is_none() || *message.last().unwrap() != self.term as u8 {
            temp = Vec::with_capacity(message.len() + 1);
            temp.extend_from_slice(message);
            temp.push(self.term as u8);
            temp.as_ref()
        } else {
            message
        };
        self.core.device_write(
            self.link_id,
            self.flags,
            self.lock_timeout,
            self.io_timeout,
            mess,
        )
    }
    pub fn device_read(&mut self) -> Result<Bytes> {
        self.core.device_read(
            self.link_id,
            self.flags,
            self.lock_timeout,
            self.io_timeout,
            self.req_size,
            self.term,
        )
    }
    pub fn device_read_str(&mut self) -> Result<String> {
        Ok(String::from_utf8_lossy(self.device_read()?.as_ref()).to_string())
    }
    pub fn device_read_stb(&mut self) -> Result<u8> {
        Ok(self.core.device_read_status(
            self.link_id,
            self.flags,
            self.lock_timeout,
            self.io_timeout,
        )? as u8)
    }
    pub fn device_enable_srq<D: AsRef<[u8]>>(&mut self, enable: bool, handle: D) -> Result<()> {
        self.core.device_enable_srq(self.link_id, enable, handle)
    }
    pub fn device_abort(&mut self) -> Result<()> {
        match self.abort {
            Some(ref mut c) => c.device_abort(self.link_id),
            None => Err(vxi11_error::Vxi11Error::NotEstablished.into()),
        }
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
    pub fn device_trigger(&mut self) -> Result<()> {
        self.core
            .device_trigger(self.link_id, self.flags, self.lock_timeout, self.io_timeout)
    }
}

impl Vxi11Client {
    pub fn connect(self, address: IpAddr, time_out: Duration) -> Result<Vxi11> {
        let mut port_mapper =
            PortMapper::new_tcp(SocketAddr::new(address, port_mapper::PORT), time_out)?;
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
