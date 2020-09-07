use arrayvec::{ArrayVec, CapacityError};
use nb::{Error, Result};

pub struct LineReader {
    buffer: ArrayVec<[u8; 256]>,
}

impl LineReader {
    pub fn new() -> LineReader {
        LineReader {
            buffer: Default::default(),
        }
    }

    pub fn push(&mut self, byte: u8) -> Result<&[u8], CapacityError<u8>> {
        if byte == b'\n' {
            Ok(self.buffer.as_slice())
        } else {
            self.buffer
                .try_push(byte)
                .map_err(|err| Error::Other(err))?;
            Err(Error::WouldBlock)
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
