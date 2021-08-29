use super::Result;
use crate::protocols::onc_rpc::{IpProtocol, Rpc, RpcProgram};
use bytes::{Bytes, BytesMut};
use serde::Serialize;
use std::{
    fmt::Debug,
    net::{IpAddr, TcpStream, ToSocketAddrs},
};

use super::{xdr, DeviceFlags, ErrorCode};
pub enum Procedure {
    ///opens a link to a device
    CreateLink,
    ///device receives a message
    DeviceWrite,
    ///device returns a result
    DeviceRead,
    ///device returns its status byte
    DeviceReadStb,
    ///device executes a trigger
    DeviceTrigger,
    ///device clears itself
    DeviceClear,
    ///device disables its front panel
    DeviceRemote,
    ///device enables its front panel
    DeviceLocal,
    ///device is locked
    DeviceLock,
    ///device is unlocked
    DeviceUnlock,
    ///device enables/disables sending of service requests
    DeviceEnableSrq,
    ///device executes a command
    DeviceDoCmd,
    ///closes a link to a device
    DestroyLink,
    ///device creates interrupt channel
    CreateIntrChan,
    ///device destroys interrupt channel
    DestroyIntrChan,
}

impl From<Procedure> for u32 {
    fn from(p: Procedure) -> Self {
        use Procedure::*;
        match p {
            CreateLink => 10,
            DeviceWrite => 11,
            DeviceRead => 12,
            DeviceReadStb => 13,
            DeviceTrigger => 14,
            DeviceClear => 15,
            DeviceRemote => 16,
            DeviceLocal => 17,
            DeviceLock => 18,
            DeviceUnlock => 19,
            DeviceEnableSrq => 20,
            DeviceDoCmd => 22,
            DestroyLink => 23,
            CreateIntrChan => 25,
            DestroyIntrChan => 26,
        }
    }
}
use Procedure::*;
pub struct Core<S> {
    io: S,
    buffer: BytesMut,
}

impl<S> RpcProgram for Core<S> {
    type IO = S;
    const PROGRAM: u32 = 0x0607AF;
    const VERSION: u32 = super::VERSION;
    fn get_io(&self) -> &Self::IO {
        &self.io
    }
    fn mut_io(&mut self) -> &mut Self::IO {
        &mut self.io
    }
    fn buffer(&self) -> BytesMut {
        self.buffer.clone()
    }
}

impl<S> Core<S> {
    pub fn new(io: S) -> Self {
        Self {
            io,
            buffer: BytesMut::new(),
        }
    }
}

impl Core<TcpStream> {
    pub fn new_tcp<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let io = TcpStream::connect(addr)?;
        Ok(Self {
            io,
            buffer: BytesMut::new(),
        })
    }
    ///return (link_id,abort_port,max_recv_size)
    pub fn create_link(
        &mut self,
        client_id: i32,
        lock: bool,
        lock_timeout: u32,
        name: String,
    ) -> Result<(i32, u32, u32)> {
        let resp: xdr::Create_LinkResp = self.call_anonymously(
            CreateLink,
            xdr::Create_LinkParms {
                clientId: xdr::long(client_id),
                lockDevice: lock,
                lock_timeout: xdr::ulong(lock_timeout),
                /*
                A TCP/IP-IEEE 488.1 Interface Device SHALL support a device string of the following format:
                 <intf_name>[,<primary_addr>[,<secondary_addr>]]
                where:
                <intf_name> A name corresponding to a single IEEE 488.1 interface. This name SHALL
                uniquely identify the interface on the TCP/IP-IEEE 488.1 Interface Device.
                <primary_addr> The primary address of a IEEE 488.1 device on the IEEE 488.1 interface (optional).
                <secondary_addr> The secondary address of a IEEE 488.1 device on the IEEE 488.1 interface (optional).
                 */
                device: name,
            },
        )?;
        Result::from(ErrorCode::from(resp.error))?;
        Ok(((resp.lid.0).0, resp.abortPort.0, resp.maxRecvSize.0))
    }

    pub fn destroy_link(&mut self, link_id: i32) -> Result<()> {
        let resp: xdr::Device_Error =
            self.call_anonymously(DestroyLink, xdr::Device_Link(xdr::long(link_id)))?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn device_write<D: AsRef<[u8]> + Debug + Serialize>(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
        io_timeout: u32,
        data: D,
    ) -> Result<usize> {
        let resp: xdr::Device_WriteResp = self.call_anonymously(
            DeviceWrite,
            xdr::Device_WriteParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                io_timeout: xdr::ulong(io_timeout),
                lock_timeout: xdr::ulong(lock_timeout),
                flags: flags.into(),
                data: unsafe { std::str::from_utf8_unchecked(data.as_ref()) },
            },
        )?;
        Result::from(ErrorCode::from(resp.error))?;
        Ok(resp.size.0 as usize)
    }
    pub fn device_read(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
        io_timeout: u32,
        req_size: usize,
        term: char,
    ) -> Result<Bytes> {
        let resp: xdr::Device_ReadResp<Bytes> = self.call_anonymously(
            DeviceRead,
            xdr::Device_ReadParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                io_timeout: xdr::ulong(io_timeout),
                lock_timeout: xdr::ulong(lock_timeout),
                requestSize: xdr::ulong(req_size as u32),
                flags: flags.into(),
                termChar: xdr::xdr_char(term as u32),
            },
        )?;
        Result::from(ErrorCode::from(resp.error))?;
        let reason = resp.reason.0;
        if (reason & (1 << 2)) == 0 {
            return Ok(resp.data);
        } else if (reason & (1 << 0)) == 0 {
            //TO-DO: requestSize transferred
            return Ok(resp.data);
        } else if reason & (1 << 1) == 0 {
            //TO-DO: terminator transferred
            return Ok(resp.data);
        } else if reason == 0 {
            return Err(super::vxi11_error::Vxi11Error::DevOutputBufFull.into());
        } else {
            unreachable!()
        }
    }
    pub fn device_read_status(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
        io_timeout: u32,
    ) -> Result<u32> {
        let resp: xdr::Device_ReadStbResp = self.call_anonymously(
            DeviceReadStb,
            xdr::Device_GenericParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                io_timeout: xdr::ulong(io_timeout),
                lock_timeout: xdr::ulong(lock_timeout),
                flags: flags.into(),
            },
        )?;
        Result::from(ErrorCode::from(resp.error))?;
        Ok(resp.stb.0)
    }
    pub fn device_trigger(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
        io_timeout: u32,
    ) -> Result<()> {
        let resp: xdr::Device_Error = self.call_anonymously(
            DeviceTrigger,
            xdr::Device_GenericParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                io_timeout: xdr::ulong(io_timeout),
                lock_timeout: xdr::ulong(lock_timeout),
                flags: flags.into(),
            },
        )?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn device_clear(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
        io_timeout: u32,
    ) -> Result<()> {
        let resp: xdr::Device_Error = self.call_anonymously(
            DeviceClear,
            xdr::Device_GenericParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                io_timeout: xdr::ulong(io_timeout),
                lock_timeout: xdr::ulong(lock_timeout),
                flags: flags.into(),
            },
        )?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn device_remote(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
        io_timeout: u32,
    ) -> Result<()> {
        let resp: xdr::Device_Error = self.call_anonymously(
            DeviceRemote,
            xdr::Device_GenericParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                io_timeout: xdr::ulong(io_timeout),
                lock_timeout: xdr::ulong(lock_timeout),
                flags: flags.into(),
            },
        )?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn device_local(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
        io_timeout: u32,
    ) -> Result<()> {
        let resp: xdr::Device_Error = self.call_anonymously(
            DeviceLocal,
            xdr::Device_GenericParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                io_timeout: xdr::ulong(io_timeout),
                lock_timeout: xdr::ulong(lock_timeout),
                flags: flags.into(),
            },
        )?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn device_lock(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
    ) -> Result<()> {
        let resp: xdr::Device_Error = self.call_anonymously(
            DeviceLock,
            xdr::Device_LockParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                lock_timeout: xdr::ulong(lock_timeout),
                flags: flags.into(),
            },
        )?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn device_unlock(&mut self, link_id: i32) -> Result<()> {
        let resp: xdr::Device_Error =
            self.call_anonymously(DeviceUnlock, &xdr::Device_Link(xdr::long(link_id)))?;
        Result::from(ErrorCode::from(resp))
    }

    pub fn create_intr_chan<A: ToSocketAddrs>(
        &mut self,
        addr: A,
        prog_num: u32,
        prog_ver: u32,
        protocol: IpProtocol,
    ) -> Result<()> {
        let protocol = match protocol {
            IpProtocol::Tcp => xdr::Device_AddrFamily::DEVICE_TCP,
            IpProtocol::Udp => xdr::Device_AddrFamily::DEVICE_UDP,
        };
        let addr = addr
            .to_socket_addrs()?
            .next()
            .expect("invalid socket address");
        let ip = match addr.ip() {
            IpAddr::V4(i) => i,
            IpAddr::V6(_) => panic!("ipv6 not supported by vxi11"),
        };
        let resp: xdr::Device_Error = self.call_anonymously(
            CreateIntrChan,
            xdr::Device_RemoteFunc {
                hostAddr: xdr::ulong(u32::from_be_bytes(ip.octets())), //not sure if big endian
                hostPort: xdr::ushort(addr.port() as u32),
                progNum: xdr::ulong(prog_num),
                progVers: xdr::ulong(prog_ver),
                progFamily: protocol,
            },
        )?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn destroy_intr_chan(&mut self) -> Result<()> {
        let resp: xdr::Device_Error = self.call_anonymously(DestroyIntrChan, ())?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn device_enable_srq<D: AsRef<[u8]> + Debug + Serialize>(
        &mut self,
        link_id: i32,
        enable: bool,
        handle: D,
    ) -> Result<()> {
        debug_assert!(handle.as_ref().len() <= 40); //handle<40>
        let resp: xdr::Device_Error = self.call_anonymously(
            DeviceEnableSrq,
            xdr::Device_EnableSrqParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                enable,
                handle: unsafe { std::str::from_utf8_unchecked(handle.as_ref()) }, //Store handle<40> so it can be passed back to the network instrument client in a device_intr_srq RPC when a service request occurs.
            },
        )?;
        Result::from(ErrorCode::from(resp))
    }
    pub fn device_do_cmd<D: AsRef<[u8]> + Debug + Serialize>(
        &mut self,
        link_id: i32,
        flags: DeviceFlags,
        lock_timeout: u32,
        io_timeout: u32,
        cmd: i32,
        network_order: bool,
        data_size: i32,
        data_in: D,
    ) -> Result<Bytes> {
        let resp: xdr::Device_DocmdResp<Bytes> = self.call_anonymously(
            DeviceDoCmd,
            xdr::Device_DocmdParms {
                lid: xdr::Device_Link(xdr::long(link_id)),
                flags: flags.into(),
                io_timeout: xdr::ulong(io_timeout),
                lock_timeout: xdr::ulong(lock_timeout),
                cmd: xdr::long(cmd),
                network_order,
                /*
                indicates the size of individual data elements A value of one(1) in datasize means byte data and no swapping is performed. A value of two(2) in datasize means 16-bit word data and bytes are swapped on word boundaries. A value of four(4) in datasize means 32-bit longword data and bytes are swapped on longword boundaries. A value of eight(8) in datasize means 64-bit data and bytes are swapped on 8-byte boundaries.
                */
                datasize: xdr::long(data_size),
                data_in: unsafe { std::str::from_utf8_unchecked(data_in.as_ref()) },
            },
        )?;
        Result::from(ErrorCode::from(resp.error))?;
        Ok(resp.data_out)
    }
}
