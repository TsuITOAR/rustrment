use std::{
    io::{BufRead, BufReader, Error, Read, Write},
    marker::PhantomData,
};

use crate::protocols::Protocol;

type Bound<P, ID> = Result<Instrument<Messenger<<P as Protocol>::IO>, ID>, <P as Protocol>::Error>;

pub mod mdt693_b;

pub trait Model {
    const DESCRIPTION: &'static str;
    type SetCommand: InstructionSet<false>;
    type QueryCommand: InstructionSet<true>;
}
pub trait InstructionSet<const REPLY: bool> {
    const TERMINATOR: u8;
    const END_BYTE: u8;
    fn to_bytes(command: Self) -> Box<[u8]>;
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
    pub fn set(&mut self, command: M::SetCommand) -> Result<(), Error> {
        let message = InstructionSet::to_bytes(command);
        self.write(&message)?;
        Ok(())
    }
    pub fn query(&mut self, command: M::QueryCommand) -> Result<String, Error> {
        let message = InstructionSet::to_bytes(command);
        self.write(&message)?;
        self.buf.clear();
        self.messenger
            .read_until(M::QueryCommand::END_BYTE, &mut self.buf)?;
        Ok(String::from_utf8_lossy(&mut self.buf).into())
    }
}
