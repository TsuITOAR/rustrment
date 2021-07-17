pub mod mdt693_b;

use crate::protocols::Protocol;
use std::{
    io::{BufRead, BufReader, Error, Read, Write},
    marker::PhantomData,
    u8,
};

type Bound<P, ID> = Result<Instrument<Messenger<<P as Protocol>::IO>, ID>, <P as Protocol>::Error>;

pub trait Model {
    const DESCRIPTION: &'static str;
    type SetCommand: InstructionSet<false>;
    type QueryCommand: InstructionSet<true>;
    const PREFIX: &'static [u8];
    const SUFFIX: &'static [u8];
    const END_BYTE: u8;
    //TO-DO: change this to Result instead of panic
    fn strip(raw: &[u8]) -> &[u8] {
        raw.strip_prefix(<Self as Model>::PREFIX)
            .expect(&format!(
                "unexpected message prefix returned by {}, expected prefix {:?}, found {:?}",
                <Self as Model>::DESCRIPTION,
                <Self as Model>::PREFIX,
                raw
            ))
            .strip_suffix(<Self as Model>::SUFFIX)
            .expect(&format!(
                "unexpected message suffix returned by {}, expected suffix {:?}, found {:?}",
                <Self as Model>::DESCRIPTION,
                <Self as Model>::SUFFIX,
                raw
            ))
    }
}
pub trait InstructionSet<const REPLY: bool> {
    const TERMINATOR: u8;
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
    pub fn query(&mut self, command: M::QueryCommand) -> Result<&[u8], Error> {
        let message = InstructionSet::to_bytes(command);
        self.write(&message)?;
        self.buf.clear();
        self.messenger.read_until(M::END_BYTE, &mut self.buf)?;
        Ok(&self.buf)
    }
}
