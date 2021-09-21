use super::Model;
use crate::DefaultConfig;

#[derive(Default)]
pub struct MDT693B;

impl DefaultConfig for MDT693B {
    type DefaultProtocol = crate::protocols::Serial;
    const DEFAULT_PROTOCOL: Self::DefaultProtocol = Self::DefaultProtocol {
        baud_rate: serial::Baud115200,
        data_bits: serial::Bits8,
        parity: serial::ParityNone,
        stop_bits: serial::Stop1,
        flow_control: serial::FlowNone,
    };
}

impl Model for MDT693B {
    const DESCRIPTION: &'static str = "Piezo controller";
    type Command = Command;
    type Query = Query;
    const TERMINATOR: u8 = b'\n';
    const END_BYTE: u8 = b']';
}

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

pub enum Command {
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

impl super::Query for Query {
    type R = &'static str;
    fn to_bytes(self) -> Self::R {
        match self {
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
    }
}

impl super::Command for Command {
    type R = Box<[u8]>;
    fn to_bytes(self) -> Self::R {
        match self {
            Command::SetEchoCommand(bo) => format!("echo={}", bo),
            Command::SetDisplayIntensity(n) => format!("intensity={}", n), //0-15
            Command::SetAllVoltages(n) => format!("allvoltage={}", n),
            Command::SetMasterScanEnable(bo) => format!("msenable={}", bo as u8),
            Command::SetMasterScanVoltage(n) => format!("msvoltage={}", n),
            Command::SetXVoltage(n) => format!("xvoltage={}", n),
            Command::SetYVoltage(n) => format!("yvoltage={}", n),
            Command::SetZVoltage(n) => format!("zvoltage={}", n),
            Command::SetMinXVoltage(n) => format!("xmin={}", n),
            Command::SetMinYVoltage(n) => format!("ymin={}", n),
            Command::SetMinZVoltage(n) => format!("zmin={}", n),
            Command::SetMaxXVoltage(n) => format!("xmax={}", n),
            Command::SetMaxYVoltage(n) => format!("ymax={}", n),
            Command::SetMaxZVoltage(n) => format!("zmax={}", n),
            Command::SetVoltageAdjustmentResolution(n) => format!("dacstep={}", n), //1-1000
            Command::IncrementVoltage => String::from_utf8(vec![0x1b, b'[', b'A']).unwrap(),
            Command::DecrementVoltage => String::from_utf8(vec![0x1b, b'[', b'B']).unwrap(),
            Command::DecreaseChannel => String::from_utf8(vec![0x1b, b'[', b'D']).unwrap(),
            Command::IncreaseChannel => String::from_utf8(vec![0x1b, b'[', b'C']).unwrap(),
            Command::SetFriendlyName(s) => format!("friendly={}", s),
            Command::SetCompatibilityMode(bo) => format!("cm={}", bo as u8),
            //Command::SetRotaryMode()=>"",//0 1 -1
            Command::SetDisableRotaryPushToAdjust(bo) => format!("disablepush={}", bo as u8),
        }
        .bytes()
        .collect::<Vec<u8>>()
        .into_boxed_slice()
    }
}

#[test]
fn stress_serial() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = MDT693B::default_connect(5)?;
    handler.query(Query::GetCommands)?;
    Ok(())
}
