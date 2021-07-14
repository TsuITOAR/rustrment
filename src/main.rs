use std::error::Error;
fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting PiezoController connecting test\n");

    let mut controller = rustrument::PiezoController::new(5)?;
    controller.set_x(10.)?;
    controller.set_y(10.)?;
    controller.set_z(10.)?;
    controller.update()?;
    println!("{}", controller);
    controller.set_x(0.)?;
    controller.update()?;
    println!("{}", controller);
    controller.set_y(0.)?;
    controller.update()?;
    println!("{}", controller);
    controller.set_z(0.)?;
    controller.update()?;
    println!("{}", controller);
    Ok(())
}
