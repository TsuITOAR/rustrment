use serial::SystemPort;

use crate::config_serial;

pub trait Protocol {
    type Address: ToString;
    type Error;
    type IO: std::io::Read + std::io::Write;
    fn connect(self, address: Self::Address) -> Result<Self::IO, Self::Error>;
}

#[derive(Clone, Copy)]
pub struct Serial {
    pub(crate) baud_rate: serial::BaudRate,
    pub(crate) data_bits: serial::CharSize,
    pub(crate) parity: serial::Parity,
    pub(crate) stop_bits: serial::StopBits,
    pub(crate) flow_control: serial::FlowControl,
}

impl Default for Serial {
    fn default() -> Self {
        Self {
            baud_rate: serial::Baud9600,
            data_bits: serial::Bits8,
            parity: serial::ParityNone,
            stop_bits: serial::Stop1,
            flow_control: serial::FlowNone,
        }
    }
}

impl Serial {
    fn format_address(address: <Self as Protocol>::Address) -> String {
        format!("COM{}", address)
    }
}

impl Protocol for Serial {
    type Address = usize;
    type Error = serial::Error;
    type IO = SystemPort;
    fn connect(self, address: Self::Address) -> Result<Self::IO, Self::Error> {
        let mut port = serial::open(&Self::format_address(address))?;
        config_serial(&mut port, self)?;
        Ok(port)
    }
}
