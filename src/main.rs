use std::error::Error;

use rustrument::instruments::mdt693_b;
use rustrument::instruments::mdt693_b::Query;
use rustrument::instruments::mdt693_b::Set;
fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting PiezoController connecting test\n");

    let mut controller = mdt693_b::new(5)?;
    //println!("{}", controller.query(Query::GetCommands)?);
    controller.set(Set::SetXVoltage(0))?;
    println!("{}", controller.query(Query::ReadXVoltage)?);
    Ok(())
}
