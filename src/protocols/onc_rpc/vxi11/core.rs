pub enum Procedure {
    ///opens a link to a device
    CreateLink,
    ///device receives a message
    DeviceWrite,
    ///device returns a result
    DeviceRead,
    ///device returns its status byte
    DeviceReadStb,
    ///device executes a trigger
    DeviceTrigger,
    ///device clears itself
    DeviceClear,
    ///device disables its front panel
    DeviceRemote,
    ///device enables its front panel
    DeviceLocal,
    ///device is locked
    DeviceLock,
    ///device is unlocked
    DeviceUnlock,
    ///device enables/disables sending of service requests
    DeviceEnableSrq,
    ///device executes a command
    DeviceDoCmd,
    ///closes a link to a device
    DestroyLink,
    ///device creates interrupt channel
    CreateIntrChan,
    ///device destroys interrupt channel
    DestroyIntrChan,
}

impl Into<u32> for Procedure {
    fn into(self) -> u32 {
        use Procedure::*;
        match self {
            CreateLink => 10,
            DeviceWrite => 11,
            DeviceRead => 12,
            DeviceReadStb => 13,
            DeviceTrigger => 14,
            DeviceClear => 15,
            DeviceRemote => 16,
            DeviceLocal => 17,
            DeviceLock => 18,
            DeviceUnlock => 19,
            DeviceEnableSrq => 20,
            DeviceDoCmd => 22,
            DestroyLink => 23,
            CreateIntrChan => 25,
            DestroyIntrChan => 26,
        }
    }
}
