use crate::protocols::Serial;

pub struct MDT693B;

pub(crate) const ID: MDT693B = MDT693B;
type DefaultProtocol = Serial;

pub const DEFAULT_PROTOCOL: DefaultProtocol = Serial {
    baud_rate: serial::Baud115200,
    data_bits: serial::Bits8,
    parity: serial::ParityNone,
    stop_bits: serial::Stop1,
    flow_control: serial::FlowNone,
};

pub enum Query {
    GetCommands,
    ProductInformation,
    GetEchoCommandValue,
    LimitSwitchSetting,
    GetDisplayIntensity,
    GetMaserScanEnable,
    ReadXVoltage,
    ReadYVoltage,
    ReadZVoltage,
    ReadMinXVoltage,
    ReadMinYVoltage,
    ReadMinZVoltage,
    ReadMaxXVoltage,
    ReadMaxYVoltage,
    ReadMaxZVoltage,
    GetVoltageAdjustmentResolution,
    GetFriendlyName,
    GetSerialNumber,
    GetCompatibility,
    GetRotaryMode,
    GetDisableRotaryPushToAdjust,
}

pub enum Set {
    SetEchoCommand(bool),
    SetDisplayIntensity(u8), //0-15
    SetAllVoltages(u8),
    SetMasterScanEnable(bool),
    SetMasterScanVoltage(u8),
    SetXVoltage(f32),
    SetYVoltage(f32),
    SetZVoltage(f32),
    SetMinXVoltage(f32),
    SetMinYVoltage(f32),
    SetMinZVoltage(f32),
    SetMaxXVoltage(f32),
    SetMaxYVoltage(f32),
    SetMaxZVoltage(f32),
    SetVoltageAdjustmentResolution(u16), //1-1000,
    IncrementVoltage,
    DecrementVoltage,
    DecreaseChannel,
    IncreaseChannel,
    SetFriendlyName(String),
    SetCompatibilityMode(bool),
    //SetRotaryMode(),//0 1 -1
    SetDisableRotaryPushToAdjust(bool),
}

impl super::Command for Query {
    type Target = MDT693B;
    type CommandType = super::QueryCommand;
    const TERMINATOR: u8 = b'\n';
    const END_BYTE: u8 = b']';
    fn to_bytes(command: Self) -> Box<[u8]> {
        match command {
            Query::GetCommands => "?",
            Query::ProductInformation => "id?",
            Query::GetEchoCommandValue => "echo?",
            Query::LimitSwitchSetting => "vlimit?",
            Query::GetDisplayIntensity => "intensity?",
            Query::GetMaserScanEnable => "msenable?",
            Query::ReadXVoltage => "xvoltage?",
            Query::ReadYVoltage => "yvoltage?",
            Query::ReadZVoltage => "zvoltage?",
            Query::ReadMinXVoltage => "xmin?",
            Query::ReadMinYVoltage => "ymin?",
            Query::ReadMinZVoltage => "zmin?",
            Query::ReadMaxXVoltage => "xmax?",
            Query::ReadMaxYVoltage => "ymax?",
            Query::ReadMaxZVoltage => "zmax?",
            Query::GetVoltageAdjustmentResolution => "dacstep?",
            Query::GetFriendlyName => "friendly?",
            Query::GetSerialNumber => "serial?",
            Query::GetCompatibility => "cm?",
            Query::GetRotaryMode => "rotarymode?",
            Query::GetDisableRotaryPushToAdjust => "disablepush?",
        }
        .bytes()
        .chain(std::iter::once(Self::TERMINATOR))
        .collect::<Vec<u8>>()
        .into_boxed_slice()
    }
}

impl super::Command for Set {
    type Target = MDT693B;
    type CommandType = super::SetCommand;
    const TERMINATOR: u8 = b'\n';
    const END_BYTE: u8 = b']';
    fn to_bytes(command: Self) -> Box<[u8]> {
        match command {
            Set::SetEchoCommand(bo) => format!("echo={}", bo),
            Set::SetDisplayIntensity(n) => format!("intensity={}", n), //0-15
            Set::SetAllVoltages(n) => format!("allvoltage={}", n),
            Set::SetMasterScanEnable(bo) => format!("msenable={}", bo as u8),
            Set::SetMasterScanVoltage(n) => format!("msvoltage={}", n),
            Set::SetXVoltage(n) => format!("xvoltage={}", n),
            Set::SetYVoltage(n) => format!("yvoltage={}", n),
            Set::SetZVoltage(n) => format!("zvoltage={}", n),
            Set::SetMinXVoltage(n) => format!("xmin={}", n),
            Set::SetMinYVoltage(n) => format!("ymin={}", n),
            Set::SetMinZVoltage(n) => format!("zmin={}", n),
            Set::SetMaxXVoltage(n) => format!("xmax={}", n),
            Set::SetMaxYVoltage(n) => format!("ymax={}", n),
            Set::SetMaxZVoltage(n) => format!("zmax={}", n),
            Set::SetVoltageAdjustmentResolution(n) => format!("dacstep={}", n), //1-1000
            Set::IncrementVoltage => "Up arrow".to_string(),
            Set::DecrementVoltage => "Down arrow".to_string(),
            Set::DecreaseChannel => "Left arrow".to_string(),
            Set::IncreaseChannel => "Right arrow".to_string(),
            Set::SetFriendlyName(s) => format!("friendly={}", s),
            Set::SetCompatibilityMode(bo) => format!("cm={}", bo as u8),
            //Set::SetRotaryMode()=>"",//0 1 -1
            Set::SetDisableRotaryPushToAdjust(bo) => format!("disablepush={}", bo as u8),
        }
        .bytes()
        .chain(std::iter::once(Self::TERMINATOR))
        .collect::<Vec<u8>>()
        .into_boxed_slice()
    }
}

pub fn new(
    address: <DefaultProtocol as super::Protocol>::Address,
) -> super::Bound<DefaultProtocol, MDT693B> {
    let channel = super::Channel::new(DEFAULT_PROTOCOL, address);
    Ok(channel.connect()?.bind(ID))
}
