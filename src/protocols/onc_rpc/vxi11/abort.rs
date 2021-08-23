pub enum Procedure {
    ///device aborts an in-progress call
    DeviceAbort,
}

impl Into<u32> for Procedure {
    fn into(self) -> u32 {
        use Procedure::*;
        match self {
            DeviceAbort => 1,
        }
    }
}
