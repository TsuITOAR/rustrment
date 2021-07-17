use std::error::Error;

use rustrument::{instruments::infiniium::Infiniium, DefaultConfig, PiezoController};
fn main() -> Result<(), Box<dyn Error>> {
    test_osc()?;
    Ok(())
}

fn test_piezo() -> Result<(), Box<dyn Error>> {
    println!("Starting PiezoController connecting test\n");

    let mut controller = PiezoController::new(5)?;
    controller.set_x(30.)?;
    controller.set_y(30.)?;
    controller.set_z(30.)?;
    controller.update()?;
    println!("{}", controller);
    Ok(())
}

fn test_osc() -> Result<(), Box<dyn Error>> {
    print!("Starting Oscilloscope connecting test\n");
    let mut osc = Infiniium::default_connect("169.254.209.174:5025".parse()?)?;
    osc.send_raw("*IDN?")?;
    println!("{}", String::from_utf8_lossy(osc.read_until(b'\n')?));
    osc.send_raw(":WAVeform:SOURce CHANnel1")?;
    osc.send_raw(":WAVeform:DATA?")?;
    println!("{}", String::from_utf8_lossy(osc.read_until(b'\n')?));
    Ok(())
}
