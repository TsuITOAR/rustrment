use super::Protocol;
use serial::SystemPort;

#[derive(Clone, Copy)]
pub struct Serial {
    pub baud_rate: serial::BaudRate,
    pub data_bits: serial::CharSize,
    pub parity: serial::Parity,
    pub stop_bits: serial::StopBits,
    pub flow_control: serial::FlowControl,
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
    type Address = u8;
    type Error = serial::Error;
    type IO = SystemPort;
    fn connect(
        self,
        address: Self::Address,
        _time_out: std::time::Duration,
    ) -> Result<Self::IO, Self::Error> {
        let mut port = serial::open(&Self::format_address(address))?;
        crate::config_serial(&mut port, self)?;
        Ok(port)
    }
}
