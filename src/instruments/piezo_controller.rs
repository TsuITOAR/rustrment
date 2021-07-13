use std::{
    io::{Error, Read, Write},
    ops::Deref,
};

use serial::SystemPort;

use crate::protocols::{Protocol, Serial};

use super::RemoteControl;

#[derive(Default)]
pub struct PiezoController {
    port: Option<SystemPort>,
}

impl Read for PiezoController {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.port {
            Some(ref mut p) => p.read(buf),
            None => Err(Error::new(
                std::io::ErrorKind::NotConnected,
                "Not Connected",
            )),
        }
    }
}
impl Write for PiezoController {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.port {
            Some(ref mut p) => p.write(buf),
            None => Err(Error::new(
                std::io::ErrorKind::NotConnected,
                "Not Connected",
            )),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self.port {
            Some(ref mut p) => p.flush(),
            None => Err(Error::new(
                std::io::ErrorKind::NotConnected,
                "Not Connected",
            )),
        }
    }
}

impl RemoteControl for PiezoController {
    type Protocol = Serial;
    const PROTOCOL: Self::Protocol = Serial {
        baud_rate: serial::Baud115200,
        data_bits: serial::Bits8,
        parity: serial::ParityNone,
        stop_bits: serial::Stop1,
        flow_control: serial::FlowNone,
    };
    fn get_io(&mut self) -> &mut Option<<Self::Protocol as Protocol>::IO> {
        &mut self.port
    }
}

impl PiezoController {
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    pub fn query<C: Deref<Target = [u8]>>(&mut self, command: C) -> std::io::Result<String> {
        let bytes_input = self.write(&command)?;
        let mut buf = vec![0u8; 128];
        let bytes_ouput = self.read(&mut buf)?;
        Ok(buf
            .into_iter()
            .take(bytes_ouput)
            .map(|c| c as char)
            .collect())
    }
}
