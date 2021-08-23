pub enum Procedure {
    ///used by device to send a service request
    DeviceIntrSrq,
}

impl Into<u32> for Procedure {
    fn into(self) -> u32 {
        use Procedure::*;
        match self {
            DeviceIntrSrq => 30,
        }
    }
}
