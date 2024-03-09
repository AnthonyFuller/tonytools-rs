use std::{
    array::TryFromSliceError,
    cmp,
    io::{self, BufRead, Write},
    iter,
    marker::PhantomData,
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
#[derive(PartialEq)]
pub enum Endianness {
    Little,
    Big,
}
pub struct ByteReader<'a> {
    buf: &'a [u8],
    pub cursor: &'a [u8],
    endianness: Endianness,
}
impl<'a> io::Read for ByteReader<'a> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        buf.write(self.buf)?;
        // probs incorrect
        Ok(cmp::min(self.cursor.len(), buf.len()))
    }
}
impl<'a> BufRead for ByteReader<'a> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(self.cursor)
    }
    fn consume(&mut self, amt: usize) {
        self.cursor = &self.cursor[amt..];
    }
}
impl<'a> ByteReader<'a> {
    pub fn iter<T: FromBytes>(&'a mut self) -> ByteReaderIterator<T>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        ByteReaderIterator::<T> {
            buf: self,
            resource_type: PhantomData,
        }
    }
    pub fn new(buf: &'a [u8], endianness: Endianness) -> Self {
        ByteReader {
            buf: buf,
            cursor: buf,
            endianness,
        }
    }
    fn read_byte(&mut self, consume: bool) -> Result<u8, ByteReaderError> {
        let res = match self.cursor.first() {
            Some(&n) => Ok(n),
            _ => Err(ByteReaderError::NoBytes),
        };
        if consume {
            self.consume(1);
        }
        res
    }
    pub fn read_cstr(&mut self) -> Result<String, ByteReaderError> {
        let mut vec: Vec<u8> = Vec::new();
        loop {
            let a = self.read_byte(true)?;
            if a == 0x00 {
                return String::from_utf8(vec).map_err(|_| ByteReaderError::NoBytes);
            } else {
                vec.push(a);
            }
        }
    }
    pub fn len(&self) -> usize {
        self.cursor.len()
    }
    pub fn cursor(&self) -> usize {
        self.buf.len() - self.cursor.len()
    }
    pub fn seek(&mut self, n: usize) -> Result<(), ByteReaderError> {
        if n > self.buf.len() {
            return Err(ByteReaderError::NoBytes);
        }
        self.cursor = &self.buf[n..];
        Ok(())
    }
    pub fn read<T: FromBytes>(&mut self) -> Result<T, ByteReaderError>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        match iter::repeat_with(|| self.read_byte(true))
            .take(size_of::<T>())
            .collect::<Result<Vec<u8>, ByteReaderError>>()
        {
            Ok(bytes) => match bytes.as_slice().try_into()
                as Result<[u8; size_of::<T>()], TryFromSliceError>
            {
                Ok(raw_bytes) => Ok(if self.endianness == Endianness::Little {
                    T::from_le_bytes(&raw_bytes.into())
                } else {
                    T::from_be_bytes(&raw_bytes.into())
                }),
                _ => Err(ByteReaderError::NoBytes),
            },
            Err(e) => Err(e),
        }
    }
    pub fn peek<T: FromBytes>(&mut self) -> Result<T, ByteReaderError>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        match iter::repeat_with(|| self.read_byte(false))
            .take(size_of::<T>())
            .collect::<Result<Vec<u8>, ByteReaderError>>()
        {
            Ok(bytes) => match bytes.as_slice().try_into()
                as Result<[u8; size_of::<T>()], TryFromSliceError>
            {
                Ok(raw_bytes) => Ok(if self.endianness == Endianness::Little {
                    T::from_le_bytes(&raw_bytes.into())
                } else {
                    T::from_be_bytes(&raw_bytes.into())
                }),
                _ => Err(ByteReaderError::NoBytes),
            },
            Err(e) => Err(e),
        }
    }
    pub fn read_n<T: FromBytes>(&mut self, n: usize) -> Result<Vec<T>, ByteReaderError>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        iter::repeat_with(|| self.read::<T>())
            .take(n)
            .collect::<Result<Vec<T>, ByteReaderError>>()
    }
    pub fn read_vec<T: FromBytes, const S: usize>(&mut self) -> Result<Vec<T>, ByteReaderError>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        let size = self.read::<u32>()? as usize;
        self.read_n::<T>(size)
    }
}

struct ByteReaderIterator<'a, T: FromBytes> {
    buf: &'a mut ByteReader<'a>,
    resource_type: PhantomData<T>,
}

impl<'a, T: FromBytes> Iterator for ByteReaderIterator<'a, T>
where
    <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.buf.read::<T>().ok()
    }
}
