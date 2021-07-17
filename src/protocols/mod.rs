pub mod serial;
pub mod tcp;
pub use self::serial::Serial;
pub trait Protocol {
    type Address: ToString;
    type Error;
    type IO: std::io::Read + std::io::Write;
    fn connect(self, address: Self::Address) -> Result<Self::IO, Self::Error>;
}
