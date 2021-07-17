pub mod infiniium;
pub mod mdt693_b;
pub mod scpi;
use crate::protocols::Protocol;
use core::str;
use std::{
    io::{BufRead, BufReader, Error, Read, Write},
    marker::PhantomData,
    u8,
};

pub(crate) type Bound<P, ID> =
    Result<Instrument<Messenger<<P as Protocol>::IO>, ID>, <P as Protocol>::Error>;

pub trait Model {
    const DESCRIPTION: &'static str;
    type Command: Command;
    type Query: Query;
    const TERMINATOR: u8;
    const END_BYTE: u8;
    //TO-DO: change this to Result instead of panic
    fn strip(raw: &[u8]) -> &[u8] {
        raw
    }
}
pub trait Command {
    type R: AsRef<[u8]>;
    fn to_bytes(self) -> Self::R;
}

pub trait Query {
    type R: AsRef<[u8]>;
    fn to_bytes(self) -> Self::R;
}

pub struct Messenger<IO: Write + Read> {
    io: IO,
}

impl<IO: Write + Read> Write for Messenger<IO> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.io.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.io.flush()
    }
}

impl<IO: Write + Read> Read for Messenger<IO> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.io.read(buf)
    }
}

impl<IO: Write + Read> Messenger<IO> {
    pub fn new(io: IO) -> Self {
        Self { io }
    }
    pub fn bind<M: Model>(self, _model: M) -> Instrument<Self, M> {
        Instrument {
            messenger: BufReader::new(self),
            model: PhantomData,
            buf: Vec::new(),
        }
    }
}

pub struct Instrument<IO: Write + Read, M: Model> {
    messenger: BufReader<IO>,
    model: PhantomData<M>,
    buf: Vec<u8>,
}

impl<IO: Write + Read, M: Model> Write for Instrument<IO, M> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.messenger.get_mut().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.messenger.get_mut().flush()
    }
}

impl<IO: Write + Read, M: Model> Read for Instrument<IO, M> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.messenger.read(buf)
    }
}

impl<IO: Write + Read, M: Model> BufRead for Instrument<IO, M> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.messenger.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.messenger.consume(amt)
    }
}

impl<IO: Write + Read, M: Model> Instrument<IO, M> {
    fn terminate_send(&mut self) -> Result<(), Error> {
        self.write(&[M::TERMINATOR])?;
        self.flush()?;
        Ok(())
    }
    pub fn command<C: Into<M::Command>>(&mut self, command: C) -> Result<(), Error> {
        let message = Command::to_bytes(command.into());
        self.write(message.as_ref())?;
        self.terminate_send()?;
        Ok(())
    }
    pub fn query<Q: Into<M::Query>>(&mut self, query: Q) -> Result<&[u8], Error> {
        let message = Query::to_bytes(query.into());
        self.write(&message.as_ref())?;
        self.terminate_send()?;
        self.buf.clear();
        self.messenger.read_until(M::END_BYTE, &mut self.buf)?;
        Ok(&self.buf)
    }
    pub fn scpi_command<C: Into<scpi::Command>>(&mut self, command: C) -> Result<(), Error>
    where
        Self: scpi::SCPI,
    {
        self.send_raw(command.into().to_bytes())?;
        Ok(())
    }
    pub fn scpi_query<Q: Into<scpi::Query>>(&mut self, query: Q) -> Result<&[u8], Error>
    where
        Self: scpi::SCPI,
    {
        self.send_raw(query.into().to_bytes())?;
        self.read_until(M::END_BYTE)
    }
    pub fn send_raw<S: AsRef<[u8]>>(&mut self, raw: S) -> Result<(), Error> {
        self.write(raw.as_ref())?;
        self.terminate_send()?;
        Ok(())
    }
    pub fn read_until(&mut self, byte: u8) -> Result<&[u8], Error> {
        self.buf.clear();
        self.messenger.read_until(byte, &mut self.buf)?;
        Ok(&self.buf)
    }
}

fn strip<'a>(raw: &'a [u8], prefix: &[u8], suffix: &[u8], model: &str) -> &'a [u8] {
    raw.strip_prefix(prefix)
        .expect(&format!(
            "unexpected message prefix returned by {}, expected prefix {:?}, found {:?}",
            model, prefix, raw
        ))
        .strip_suffix(suffix)
        .expect(&format!(
            "unexpected message suffix returned by {}, expected suffix {:?}, found {:?}",
            model, suffix, raw
        ))
}
