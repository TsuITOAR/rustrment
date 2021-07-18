use instruments::{mdt693_b::MDT693B, Instrument, Messenger, Model};
use protocols::{Protocol, Serial};
use serial::SerialPort;
use std::{error::Error, fmt::Display};

pub mod instruments;
pub mod protocols;

pub trait DefaultConfig: Model + Sized + Default {
    type DefaultProtocol: Protocol;
    const DEFAULT_PROTOCOL: Self::DefaultProtocol;
    fn default_connect(
        address: <Self::DefaultProtocol as Protocol>::Address,
    ) -> instruments::Bound<Self::DefaultProtocol, Self> {
        let messenger = Messenger::new(Self::DEFAULT_PROTOCOL.connect(address)?);
        Ok(messenger.bind(Self::default()))
    }
}

fn config_serial<T: SerialPort>(port: &mut T, config: Serial) -> serial::Result<()> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(config.baud_rate)?;
        settings.set_char_size(config.data_bits);
        settings.set_parity(config.parity);
        settings.set_stop_bits(config.stop_bits);
        settings.set_flow_control(config.flow_control);
        Ok(())
    })
}

pub struct PiezoController {
    x_voltage: f32,
    y_voltage: f32,
    z_voltage: f32,
    messenger: Instrument<Messenger<<Serial as Protocol>::IO>, MDT693B>,
    flag: (bool, bool, bool),
    time_set: std::time::Instant,
}

impl PiezoController {
    fn extract_num(message: &[u8]) -> Result<f32, Box<dyn Error>> {
        Ok(std::str::from_utf8({
            let temp = message
                .split(|x| *x == b'\n' || *x == b'\r')
                .last()
                .ok_or::<Box<dyn Error>>("no line received".into())?;
            let mut iter = temp
                .iter()
                .enumerate()
                .skip_while(|x| !(*(*x).1 as char).is_numeric())
                .take_while(|x| *(*x).1 != b']');
            if let Some((start, _)) = iter.next() {
                if let Some((end, _)) = iter.last() {
                    &temp[start..end + 1]
                } else {
                    &temp[start..start + 1]
                }
            } else {
                return Err("no number found".into());
            }
        })?
        .parse()?)
    }

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        use instruments::mdt693_b::Query;
        if self.flag == (true, true, true) {
            return Ok(());
        } else {
            let (x, y, z) = (self.x_voltage, self.y_voltage, self.z_voltage);
            while (((x == self.x_voltage) && !self.flag.0)
                || ((y == self.y_voltage) && !self.flag.1)
                || ((z == self.z_voltage) && !self.flag.2))
                && std::time::Instant::now() - self.time_set
                    <= std::time::Duration::new(0, 1000_000_000)
            {
                self.x_voltage = Self::extract_num(self.messenger.query(Query::ReadXVoltage)?)?;
                self.y_voltage = Self::extract_num(self.messenger.query(Query::ReadYVoltage)?)?;
                self.z_voltage = Self::extract_num(self.messenger.query(Query::ReadZVoltage)?)?;
            }
            self.flag = (true, true, true);
            Ok(())
        }
    }
    pub fn new(address: <Serial as Protocol>::Address) -> Result<Self, Box<dyn Error>> {
        use instruments::mdt693_b::Query;

        let mut messenger = instruments::mdt693_b::MDT693B::default_connect(address)?;
        let x_voltage = Self::extract_num(messenger.query(Query::ReadXVoltage)?)?;
        let y_voltage = Self::extract_num(messenger.query(Query::ReadYVoltage)?)?;
        let z_voltage = Self::extract_num(messenger.query(Query::ReadZVoltage)?)?;
        Ok(Self {
            x_voltage,
            y_voltage,
            z_voltage,
            messenger,
            flag: (true, true, true),
            time_set: std::time::Instant::now(),
        })
    }
    pub fn messenger(&mut self) -> &mut Instrument<Messenger<<Serial as Protocol>::IO>, MDT693B> {
        &mut self.messenger
    }
    pub fn set_x(&mut self, voltage: f32) -> Result<(), Box<dyn Error>> {
        use instruments::mdt693_b::Command;
        self.messenger.command(Command::SetXVoltage(voltage))?;
        self.time_set = std::time::Instant::now();
        self.flag.0 = false;
        Ok(())
    }
    pub fn set_y(&mut self, voltage: f32) -> Result<(), Box<dyn Error>> {
        use instruments::mdt693_b::Command;
        self.messenger.command(Command::SetYVoltage(voltage))?;
        self.time_set = std::time::Instant::now();
        self.flag.1 = false;
        Ok(())
    }
    pub fn set_z(&mut self, voltage: f32) -> Result<(), Box<dyn Error>> {
        use instruments::mdt693_b::Command;
        self.messenger.command(Command::SetZVoltage(voltage))?;
        self.time_set = std::time::Instant::now();
        self.flag.2 = false;
        Ok(())
    }
    pub fn x(&self) -> f32 {
        self.x_voltage
    }
    pub fn y(&self) -> f32 {
        self.y_voltage
    }
    pub fn z(&self) -> f32 {
        self.z_voltage
    }
    //real time voltage
    pub fn rt_x(&mut self) -> Result<f32, Box<dyn Error>> {
        self.update()?;
        Ok(self.x_voltage)
    }
    pub fn rt_y(&mut self) -> Result<f32, Box<dyn Error>> {
        self.update()?;
        Ok(self.y_voltage)
    }
    pub fn rt_z(&mut self) -> Result<f32, Box<dyn Error>> {
        self.update()?;
        Ok(self.z_voltage)
    }
}

impl Display for PiezoController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "X\tY\tZ\t\n{}\t{}\t{}",
            self.x_voltage, self.y_voltage, self.z_voltage
        )
    }
}
