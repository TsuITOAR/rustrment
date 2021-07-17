use std::{
    error::Error,
    io::{Read, Write},
};

use rustrument::{
    protocols::{self, Protocol},
    PiezoController,
};
fn main() -> Result<(), Box<dyn Error>> {
    test_oscill()?;
    Ok(())
}

fn test_piezo_controller() -> Result<(), Box<dyn Error>> {
    println!("Starting PiezoController connecting test\n");

    let mut controller = PiezoController::new(5)?;
    controller.set_x(30.)?;
    controller.set_y(30.)?;
    controller.set_z(30.)?;
    controller.update()?;
    println!("{}", controller);
    Ok(())
}

fn test_oscill() -> Result<(), Box<dyn Error>> {
    print!("Starting Oscilloscope connecting test\n");
    let mut oscil = protocols::tcp::Tcp.connect("169.254.209.174:5025".parse()?)?;
    oscil.write("*IDN?\n".as_ref())?;
    oscil.flush()?;
    let mut buf = [0u8; 128];
    oscil.set_read_timeout(Some(std::time::Duration::new(1, 0)))?;
    oscil.read(&mut buf)?;
    println!("{}", String::from_utf8_lossy(&buf));
    Ok(())
}
