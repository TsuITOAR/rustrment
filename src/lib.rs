use std::{error::Error, fmt::Display};

use instruments::{mdt693_b::MDT693B, Instrument, Messenger, Model};
use protocols::{Protocol, Serial};
use serial::SerialPort;

pub mod instruments;
pub mod protocols;

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
        Ok(std::str::from_utf8(<MDT693B as Model>::strip(message))?.parse()?)
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

        let mut messenger = instruments::mdt693_b::new(address)?;
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
        use instruments::mdt693_b::Set;
        self.messenger.set(Set::SetXVoltage(voltage))?;
        self.time_set = std::time::Instant::now();
        self.flag.0 = false;
        Ok(())
    }
    pub fn set_y(&mut self, voltage: f32) -> Result<(), Box<dyn Error>> {
        use instruments::mdt693_b::Set;
        self.messenger.set(Set::SetYVoltage(voltage))?;
        self.time_set = std::time::Instant::now();
        self.flag.1 = false;
        Ok(())
    }
    pub fn set_z(&mut self, voltage: f32) -> Result<(), Box<dyn Error>> {
        use instruments::mdt693_b::Set;
        self.messenger.set(Set::SetZVoltage(voltage))?;
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
