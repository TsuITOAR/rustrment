pub mod error;
pub trait Scpi{
    fn scpi_send<C:AsRef<[u8]>>(&mut self,command:C);

}

pub enum ScpiCommonCommands{

}