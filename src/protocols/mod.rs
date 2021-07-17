pub mod serial;
pub mod tcp;

pub use self::serial::Serial;
pub use self::tcp::Tcp;
pub trait Protocol {
    type Address;
    type Error;
    type IO: std::io::Read + std::io::Write;
    fn connect(self, address: Self::Address) -> Result<Self::IO, Self::Error>;
}
