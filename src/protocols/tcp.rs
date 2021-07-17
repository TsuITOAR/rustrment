use super::Protocol;
use std::{
    io::Error,
    net::{SocketAddr, TcpStream},
};
pub struct Tcp;

impl Default for Tcp {
    fn default() -> Self {
        Tcp
    }
}

impl Protocol for Tcp {
    type IO = TcpStream;
    type Address = SocketAddr;
    type Error = Error;
    fn connect(self, address: Self::Address) -> Result<Self::IO, Self::Error> {
        TcpStream::connect(address)
    }
}
