//! bytereader.rs
use std::{
    array::TryFromSliceError,
    cmp, default,
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
#[derive(PartialEq, Clone, Default)]
pub enum Endianness {
    #[default]
    Little,
    Big,
}

/// A tool for reading bytes from a buffer
#[derive(Clone)]
pub struct ByteReader<'a> {
    /// A reference to the data that is being read.
    buf: &'a [u8],
    /// The current place in the buffer.
    pub cursor: &'a [u8],
    /// The endianness in which the bytes should be read as.
    endianness: Endianness,
}

impl<'a> ByteReader<'a> {
    /// Returns a ByteReader reading from buf
    ///
    /// # Arguments
    ///
    /// * `buf` - A slice of u8, the buffer of bytes
    /// * `endianness` - The endianness in which the bytes should be read
    ///
    /// # Examples
    /// ```
    /// use crate::util::bytereader;
    /// // get buffer from file
    /// let buf = std::fs::read("binary.file")?.as_slice();
    /// let reader = ByteReader::new(buf, Endianness::default());
    /// // alternatively:
    /// let reader = ByteReader::new(buf, Endianness::Big));
    /// ```
    pub fn new(buf: &'a [u8], endianness: Endianness) -> Self {
        ByteReader {
            buf: buf,
            cursor: buf,
            endianness,
        }
    }

    /// Returns a ByteReaderIterator<T> that iterates over a buffer, returing bytes of type T
    ///
    /// # Examples
    /// ```
    /// use crate::util::bytereader;
    /// // get buffer from file
    /// let buf = std::fs::read("values.file")?.as_slice();
    /// let reader = ByteReader::new(buf, Endianness::Little);
    /// let values_squared = reader.iter::<u16>().map(|t|t*t).collect();
    /// ```
    pub fn iter<T: FromBytes>(&'a mut self) -> ByteReaderIterator<T>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        ByteReaderIterator::<T> {
            buf: self,
            resource_type: PhantomData,
        }
    }

    /// Peeks one byte into the buffer without consuming it
    fn peek_byte(&self) -> Result<u8, ByteReaderError> {
        self.cursor
            .first()
            .ok_or_else(|| ByteReaderError::NoBytes)
            .copied()
    }
    /// Reads one byte from the buffer, consuming it
    ///
    /// # Examples
    /// ```
    /// use crate::util::bytereader;
    /// let buf = std::fs::read("binary.file")?.as_slice();
    /// let reader = ByteReader::new(buf, Endianness::Little);
    /// let first: u8 = reader.peek_byte()?;
    /// ```
    fn read_byte(&mut self) -> Result<u8, ByteReaderError> {
        let res = self
            .cursor
            .first()
            .ok_or_else(|| ByteReaderError::NoBytes)
            .copied();
        self.consume(1);
        res
    }

    // would like this to be read::<&str>() and read::<std::string>()
    pub fn read_cstr(&mut self) -> Result<String, ByteReaderError> {
        let mut vec: Vec<u8> = Vec::new();
        loop {
            let a = self.read_byte()?;
            if a == 0x00 {
                return String::from_utf8(vec).map_err(|_| ByteReaderError::NoBytes);
            } else {
                vec.push(a);
            }
        }
    }

    /// Returns the length of the remaining buffer
    pub fn len(&self) -> usize {
        self.cursor.len()
    }

    /// Returns the cursor position
    pub fn cursor(&self) -> usize {
        self.buf.len() - self.cursor.len()
    }

    /// Seeks to a position in the buffer
    ///
    /// # Arguments
    ///
    /// * `pos` - the position in the buffer
    ///
    /// # Examples
    /// ```
    /// use crate::util::bytereader;
    /// let buf = std::fs::read("binary.file")?.as_slice();
    /// let reader = ByteReader::new(buf, Endianness::Little);
    /// if reader.seek(15).is_ok() {
    ///     reader.read::<u32>();
    /// }
    /// ```
    pub fn seek(&mut self, pos: usize) -> Result<(), ByteReaderError> {
        if pos > self.buf.len() {
            return Err(ByteReaderError::NoBytes);
        }
        self.cursor = &self.buf[pos..];
        Ok(())
    }

    /// Reads a type T from the buffer
    ///
    /// # Arguments
    ///
    /// * `T: FromBytes` - the type you want to read
    ///
    /// # Examples
    /// ```
    /// use crate::util::bytereader;
    ///
    /// // get buffer
    /// let buf = std::fs::read("binary.file")?.as_slice();
    /// let reader = ByteReader::new(buf, Endianness::Little);
    ///
    /// let value = reader.read::<u32>()?;
    /// // only reads 2 bytes!
    /// let next_value = reader.read::<u16>()? as u32;
    /// ```
    pub fn read<T: FromBytes>(&mut self) -> Result<T, ByteReaderError>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        match iter::repeat_with(|| self.read_byte())
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

    /// Reads a type T from the buffer
    ///
    /// # Arguments
    ///
    /// * `T: FromBytes` - the type you want to read
    ///
    /// # Examples
    /// ```
    /// use crate::util::bytereader;
    ///
    /// // get buffer
    /// let buf = std::fs::read("binary.file")?.as_slice();
    /// let reader = ByteReader::new(buf, Endianness::Little);
    ///
    /// // doesn't consume the next 4 bytes!
    /// let full_value = reader.peek::<u32>()?;
    /// // does consume the next 4 bytes!
    /// let [first_half, second_half] = [reader.read::<u16>()? as u32, reader.read::<u16>()? as u32];
    /// ```
    pub fn peek<T: FromBytes>(&mut self) -> Result<T, ByteReaderError>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        let here = self.cursor;
        let res = match iter::repeat_with(|| self.read_byte())
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
        };
        self.cursor = here;
        res
    }

    /// Reads a type T from the buffer
    ///
    /// # Arguments
    ///
    /// * `T: FromBytes` - the type you want to read
    ///
    /// # Examples
    /// ```
    /// use crate::util::bytereader;
    ///
    /// // get buffer
    /// let buf = std::fs::read("binary.file")?.as_slice();
    /// let reader = ByteReader::new(buf, Endianness::Little);
    ///
    /// // read 32 bytes (or 16 u16s)
    /// let values = reader.read_n::<u16>(16)?;
    /// ```
    pub fn read_n<T: FromBytes>(&mut self, n: usize) -> Result<Vec<T>, ByteReaderError>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        iter::repeat_with(|| self.read::<T>())
            .take(n)
            .collect::<Result<Vec<T>, ByteReaderError>>()
    }

    // would like this to possibly be: read::<Vec<T>>()
    pub fn read_vec<T: FromBytes>(&mut self) -> Result<Vec<T>, ByteReaderError>
    where
        <T as FromBytes>::Bytes: From<[u8; size_of::<T>()]>,
    {
        let size = self.read::<u32>()? as usize;
        self.read_n::<T>(size)
    }
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

pub struct ByteReaderIterator<'a, T: FromBytes> {
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
