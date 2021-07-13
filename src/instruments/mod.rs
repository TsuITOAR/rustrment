use crate::protocols::Protocol;

pub mod piezo_controller;

pub trait RemoteControl {
    type Protocol: Protocol;
    const PROTOCOL: Self::Protocol;
    fn get_io(&mut self) -> &mut Option<<Self::Protocol as Protocol>::IO>;
    fn connect(
        &mut self,
        address: <Self::Protocol as Protocol>::Address,
    ) -> Result<(), <Self::Protocol as Protocol>::Error> {
        *self.get_io() = Some(Self::PROTOCOL.connect(address)?);
        Ok(())
    }
}
