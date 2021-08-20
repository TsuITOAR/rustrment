#![allow(dead_code)]
use std::{
    error::Error,
    io::{BufRead, Read, Write},
    net::SocketAddr,
};

use rustrument::{
    instruments::{infiniium::Infiniium, Messenger},
    protocols::{Protocol, Tcp},
    DefaultConfig, PiezoController,
};

fn main() -> Result<(), Box<dyn Error>> {
    test_port_mapper::<std::net::IpAddr>("192.168.3.96".parse()?)?;
    Ok(())
}
fn test_port_mapper<A: Into<std::net::IpAddr>>(addr: A) -> Result<(), Box<dyn Error>> {
    use rustrument::protocols::onc_rpc::{port_mapper::*, *};
    use serde_xdr::{from_bytes, to_bytes};
    let mut handler = PortMapper::new_udp(1000)?;
    let reply: u32 = from_bytes(handler.call_to_anonymously(
        Procedure::GetPort,
        to_bytes(&xdr::mapping {
            port: 0,
            prog: 0x0607AF,
            prot: xdr::IPPROTO_TCP,
            vers: 1,
        })?,
        SocketAddr::new(addr.into(), PORT),
    )?)?;
    println!("got reply: {}", reply);
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
    println!(
        "{:?}",
        std::str::from_utf8(osc.read_until(b'\n')?)?
            .lines()
            .next()
            .expect("no data received")
            .split_terminator(',')
            .map(|x| -> f32 { x.parse().unwrap() })
            .collect::<Vec<f32>>()
    );
    osc.send_raw(":ACQuire:SRATe:ANALog 250E+6")?;
    osc.send_raw("STOP")?;
    Ok(())
}

fn test_osc_rigol() -> Result<(), Box<dyn Error>> {
    let m = Tcp::default();
    let mut mess =
        std::io::BufReader::new(Messenger::new(m.connect("169.254.120.131:5555".parse()?)?));
    println!("connect success");
    mess.get_mut().write(":WAVeform:FORMat ASCii\n".as_ref())?;
    mess.get_mut().write(":WAVeform:MODE MAXimum\n".as_ref())?;
    for i in [1].iter() {
        mess.get_mut()
            .write(format!(":WAVeform:STOP {}\n", i * 1000_000).as_ref())?;
        mess.get_mut().write(":WAVeform:DATA?\n".as_ref())?;
        let mut buf = Vec::new();
        mess.read_until(b'\n', &mut buf)?;
        println!("read success");
        println!(
            "{}\n{}",
            String::from_utf8_lossy(buf.as_ref()),
            buf.len() / 14,
        );
    }

    Ok(())
}

fn test_awg_rigol() -> Result<(), Box<dyn Error>> {
    let m = Tcp;
    let mut mess = std::io::BufReader::new(Messenger::new(m.connect("192.168.3.94:111".parse()?)?));
    println!("connect success");

    let mut buf = [0_u8; 4];
    mess.read(&mut buf)?;
    println!("{}", u32::from_be_bytes(buf));
    Ok(())
}
