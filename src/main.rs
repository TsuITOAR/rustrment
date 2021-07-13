use std::{
    error::Error,
    io::{self, BufReader, Read, Write},
};

use instruemnt_controller::instruments::{piezo_controller, RemoteControl};

fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting PiezoController connecting test\n");
    let mut controller = piezo_controller::PiezoController::new();
    controller.connect(5)?;
    let mut buf = vec![1; 128000];
    let mut buf_con = BufReader::new(controller);
    buf_con.get_mut().write("?\r\n".as_bytes())?;
    buf_con.read(&mut buf)?;
    println!("{}", buf.iter().map(|x| *x as char).collect::<String>());
    buf = vec![1; 128000];
    let mut guess: String = "".to_string();
    loop {
        let n = io::stdin()
            .read_line(&mut guess)
            .expect("Failed to read line");
        if n <= 2 {
            break;
        }
        buf_con
            .get_mut()
            .write(format!("xvoltage={}\r\n", guess).as_bytes())?;
        buf_con.read(&mut buf)?;

        println!("{}", buf.iter().map(|x| *x as char).collect::<String>());
        guess.clear();
        buf = vec![1; 128000];
    }

    Ok(())
}
