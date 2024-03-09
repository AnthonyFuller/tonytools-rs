use std::mem::size_of;

use num::traits::ToBytes;

use super::bytereader::Endianness;

pub enum ByteWriterError {
    Fail,
}

pub struct ByteWriter {
    buf: Vec<u8>,
    endianness: Endianness,
}

impl ByteWriter {
    pub fn write<T: ToBytes>(&mut self, data: T) -> Result<(), ByteWriterError>
    where
        <T as ToBytes>::Bytes: Into<Vec<u8>>,
    {
        let mut buf: Vec<u8> = if self.endianness == Endianness::Little {
            data.to_le_bytes().into()
        } else {
            data.to_be_bytes().into()
        };
        self.buf.append(&mut buf);
        Ok(())
    }
}
