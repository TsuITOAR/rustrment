use std::{
    io::{BufRead, BufReader, Error, Read, Write},
    marker::PhantomData,
};

use crate::protocols::Protocol;

pub type Bound<P, ID> =
    Result<Instrument<Messenger<<P as Protocol>::IO>, ID>, <P as Protocol>::Error>;

pub mod mdt693_b;
pub trait Command {
    type Target;
    type CommandType;
    const TERMINATOR: u8;
    const END_BYTE: u8;
    fn to_bytes(command: Self) -> Box<[u8]>;
}

pub struct SetCommand;
pub struct QueryCommand;

pub struct Channel<P: Protocol> {
    protocol: P,
    address: <P as Protocol>::Address,
}

impl<P: Protocol> Channel<P> {
    pub fn new(p: P, address: <P as Protocol>::Address) -> Self {
        Self {
            protocol: p,
            address,
        }
    }
    pub fn connect(self) -> Result<Messenger<<P as Protocol>::IO>, <P as Protocol>::Error> {
        let io = self.protocol.connect(self.address)?;
        Ok(Messenger { io })
    }
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
    pub fn bind<T>(self, _target: T) -> Instrument<Self, T> {
        Instrument {
            messenger: BufReader::new(self),
            target_marker: PhantomData,
            buf: Vec::new(),
        }
    }
}

pub struct Instrument<IO: Write + Read, T> {
    messenger: BufReader<IO>,
    target_marker: PhantomData<T>,
    buf: Vec<u8>,
}

impl<IO: Write + Read, T> Write for Instrument<IO, T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.messenger.get_mut().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.messenger.get_mut().flush()
    }
}

impl<IO: Write + Read, T> Read for Instrument<IO, T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.messenger.read(buf)
    }
}

impl<IO: Write + Read, T> BufRead for Instrument<IO, T> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.messenger.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.messenger.consume(amt)
    }
}

impl<IO: Write + Read, T> Instrument<IO, T> {
    pub fn set<S>(&mut self, command: S) -> Result<(), Error>
    where
        S: Command<CommandType = SetCommand, Target = T>,
    {
        let message = Command::to_bytes(command);
        self.write(&message)?;
        Ok(())
    }
    pub fn query<Q>(&mut self, command: Q) -> Result<String, Error>
    where
        Q: Command<CommandType = QueryCommand, Target = T>,
    {
        let message = Command::to_bytes(command);
        self.write(&message)?;
        self.buf.clear();
        self.messenger.read_until(Q::END_BYTE, &mut self.buf)?;
        Ok(String::from_utf8_lossy(&mut self.buf).into())
    }
}
