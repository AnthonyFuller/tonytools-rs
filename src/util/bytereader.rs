use std::{
    array::TryFromSliceError,
    cmp,
    io::{self, BufRead, Write},
    iter,
    mem::size_of,
};

use num::traits::FromBytes;

/// Error returned by ByteReader
#[derive(Debug)]
pub enum ByteReaderError {
    NoBytes,
    IOError(io::Error),
}

impl From<std::io::Error> for ByteReaderError {
    fn from(err: std::io::Error) -> Self {
        ByteReaderError::IOError(err)
    }
}

pub struct ByteReader<'a> {
    buf: &'a [u8],
}
impl<'a> io::Read for ByteReader<'a> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        buf.write(self.buf)?;
        Ok(cmp::min(self.buf.len(), buf.len()))
    }
}
impl<'a> BufRead for ByteReader<'a> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(self.buf)
    }
    fn consume(&mut self, amt: usize) {
        self.buf = &self.buf[amt..];
    }
}
impl<'a> ByteReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        ByteReader { buf: buf }
    }
    fn read_byte(&mut self) -> Result<u8, ByteReaderError> {
        let res = match self.buf.first() {
            Some(&n) => Ok(n),
            _ => Err(ByteReaderError::NoBytes),
        };
        self.consume(1);
        res
    }
    pub fn read<T: FromBytes, const S: usize>(&mut self) -> Result<T, ByteReaderError>
    where
        T::Bytes: From<[u8; S]>,
    {
        assert_eq!(S, size_of::<T>());
        match iter::repeat_with(|| self.read_byte())
            .take(S)
            .collect::<Result<Vec<u8>, ByteReaderError>>()
        {
            Ok(bytes) => match bytes.as_slice().try_into() as Result<[u8; S], TryFromSliceError> {
                Ok(raw_bytes) => Ok(T::from_le_bytes(&raw_bytes.into())),
                _ => Err(ByteReaderError::NoBytes),
            },
            Err(e) => Err(e),
        }
    }
    pub fn read_n<T: FromBytes, const S: usize>(
        &mut self,
        n: usize,
    ) -> Result<Vec<T>, ByteReaderError>
    where
        T::Bytes: From<[u8; S]>,
    {
        iter::repeat_with(|| self.read::<T, S>())
            .take(n)
            .collect::<Result<Vec<T>, ByteReaderError>>()
    }
}
