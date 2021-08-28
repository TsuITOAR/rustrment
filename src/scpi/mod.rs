use bytes::{Buf, Bytes};
pub mod com_cmd;
pub mod error;
type Result<T> = std::result::Result<T, error::ScpiError>;
pub trait Scpi {
    fn scpi_send<C: AsRef<[u8]>>(&mut self, command: C) -> Result<()>;
    fn scpi_read(&mut self) -> Result<Bytes>;
    fn get_event_byte(&mut self) -> Result<EventStatusByte> {
        self.scpi_send(com_cmd::ESR.to_command().query())?;
        let mut b = self.scpi_read()?;
        let byte = EventStatusByte::new(b.get_u8());
        Ok(byte)
    }
    fn get_status_byte(&mut self) -> Result<StatusByte> {
        self.scpi_send(com_cmd::STB.to_command().query())?;
        let mut b = self.scpi_read()?;
        let byte = StatusByte::new(b.get_u8());
        Ok(byte)
    }
    fn set_event_mask(&mut self, byte: EventStatusByte) -> Result<()> {
        self.scpi_send(com_cmd::ESR.to_command().para(byte.to_string()))
    }
    fn set_service_mask(&mut self, byte:StatusByte)->Result<()>{
        self.scpi_send(com_cmd::SRE.to_command().para(byte.to_string()))
    }
}
#[derive(Debug)]
pub struct Command(String);

impl Command {
    pub fn new<S: ToString>(s: S) -> Self {
        Self(s.to_string())
    }
    pub fn query(&mut self) -> &mut Self {
        self.0.push('?');
        self
    }
    pub fn para<P: AsRef<str>>(&mut self, para: P) -> &mut Self {
        self.0.push(' ');
        self.0.push_str(para.as_ref());
        self
    }
    pub fn into_inner(self) -> String {
        self.0
    }
}
impl AsRef<[u8]> for Command {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}
impl AsRef<str> for Command {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}
impl<T: ToString> From<T> for Command {
    fn from(s: T) -> Self {
        Self(s.to_string())
    }
}

pub trait ToCommand {
    fn to_command(&self) -> Command;
}
impl<T> ToCommand for T
where
    T: ToString,
{
    fn to_command(&self) -> Command {
        Command(self.to_string())
    }
}

pub struct StatusByte(u8);
impl StatusByte {
    pub fn new(b: u8) -> Self {
        Self(b)
    }
    pub fn byte(&self) -> u8 {
        self.0
    }

    pub fn is_triggered(&self) -> bool {
        self.0 & (1 << 0) == 0
    }
    pub fn triggered(&mut self) -> &mut Self {
        self.0 &= 1 << 0;
        self
    }
    pub fn is_displaying_message(&self) -> bool {
        self.0 & (1 << 2) == 0
    }
    pub fn displaying_message(&mut self) -> &mut Self {
        self.0 &= 1 << 2;
        self
    }
    pub fn is_message_available(&self) -> bool {
        self.0 & (1 << 4) == 0
    }
    pub fn message_available(&mut self) -> &mut Self {
        self.0 &= 1 << 4;
        self
    }

    pub fn is_event_happened(&self) -> bool {
        self.0 & (1 << 5) == 0
    }
    pub fn event_happened(&mut self) -> &mut Self {
        self.0 &= 1 << 5;
        self
    }
    pub fn is_requesting_service(&self) -> bool {
        self.0 & (1 << 6) == 0
    }
}

pub struct EventStatusByte(u8);
impl EventStatusByte {
    pub fn new(b: u8) -> Self {
        Self(b)
    }
    pub fn byte(&self) -> u8 {
        self.0
    }
    pub fn is_command_err(&self) -> bool {
        self.0 & (1 << 5) != 0
    }
    pub fn command_err(&mut self) -> &mut Self {
        self.0 &= 1 << 5;
        self
    }
    pub fn is_device_dep_err(&self) -> bool {
        self.0 & (1 << 3) != 0
    }
    pub fn device_dep_err(&mut self) -> &mut Self {
        self.0 &= 1 << 3;
        self
    }
    pub fn is_query_err(&self) -> bool {
        self.0 & (1 << 2) != 0
    }
    pub fn query_err(&mut self) -> &mut Self {
        self.0 &= 1 << 2;
        self
    }
    pub fn is_opera_complete(&self) -> bool {
        self.0 & (1 << 0) != 0
    }
    pub fn opera_complete(&mut self) -> &mut Self {
        self.0 &= 1 << 0;
        self
    }
}
impl ToString for StatusByte {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
impl ToString for EventStatusByte {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
