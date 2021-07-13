use std::error::Error;

use rustrument::instruments::{piezo_controller, RemoteControl};

fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting PiezoController connecting test\n");
    
    let mut controller = piezo_controller::PiezoController::new();
    controller.connect(5)?;
    println!("{:?}", controller.query("xvoltage?\n".as_bytes())?);

    Ok(())
}
