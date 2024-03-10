//! bytereader.rs
use std::{
    cmp,
    convert::Infallible,
    io::{self, BufRead, Write},
    iter,
    marker::PhantomData,
};

use crate::util::transmutable::{Endianness, TryFromBytes, TryFromBytesError};


pub trait ByteReaderResource = TryFromBytes<Bytes = Vec<u8>, Error = TryFromBytesError>;
/// Error returned by ByteReader
pub struct ByteReaderError {
    kind: ByteReaderErrorKind,
    cursor: usize,
}

impl std::fmt::Debug for ByteReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ByteReaderError")
            .field("kind", &self.kind)
            .field_with("cursor", |f| write!(f, "{:#x}", &self.cursor))
            .finish()
    }
}
#[derive(Debug)]
pub enum ByteReaderErrorKind {
    NoBytes,
    TryFromBytesError(TryFromBytesError),
    IOError(io::Error),
    Infallible,
}

impl From<std::io::Error> for ByteReaderErrorKind {
    fn from(err: std::io::Error) -> Self {
        ByteReaderErrorKind::IOError(err)
    }
}

impl From<TryFromBytesError> for ByteReaderErrorKind {
    fn from(err: TryFromBytesError) -> Self {
        ByteReaderErrorKind::TryFromBytesError(err)
    }
}

impl From<Infallible> for ByteReaderErrorKind {
    fn from(_: Infallible) -> Self {
        ByteReaderErrorKind::Infallible
    }
}

impl From<Infallible> for ByteReaderError {
    fn from(_: Infallible) -> Self {
        ByteReaderError {
            kind: ByteReaderErrorKind::Infallible,
            cursor: 20, // OK ??
        }
    }
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
    /// Returns a ByteReaderError with the context of the ByteReader
    ///
    /// # Arguments
    ///
    /// * `kind` - the kind of error to receive
    pub fn err(&self, kind: ByteReaderErrorKind) -> ByteReaderError {
        ByteReaderError {
            kind,
            cursor: self.cursor(),
        }
    }
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
    pub fn iter<T: TryFromBytes>(&'a mut self) -> ByteReaderIterator<T>
    where
        T::Bytes: TryFrom<Vec<u8>>,
    {
        ByteReaderIterator::<T> {
            buf: self,
            resource_type: PhantomData,
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
            return Err(self.err(ByteReaderErrorKind::NoBytes));
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
    pub fn read<T: ByteReaderResource>(&mut self) -> Result<T, ByteReaderError> {
        let (v, s) = T::try_from_bytes(self.cursor.to_vec(), self.endianness)
            .map_err(|e| self.err(ByteReaderErrorKind::TryFromBytesError(e)))?;
        self.consume(s);
        Ok(v)
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
    pub fn peek<T: ByteReaderResource>(&mut self) -> Result<T, ByteReaderError> {
        let here = self.cursor;
        let res = self.read::<T>();
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
    pub fn read_n<T: ByteReaderResource>(&mut self, n: usize) -> Result<Vec<T>, ByteReaderError> {
        iter::repeat_with(|| self.read::<T>())
            .take(n)
            .collect::<Result<Vec<T>, ByteReaderError>>()
    }

    /// Reads a type T from the buffer n times without consuming
    ///
    /// # Arguments
    ///
    /// * `T: FromBytes` - the type you want to read
    /// * `n` - the number of T to read
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
    pub fn peek_n<T: ByteReaderResource>(&mut self, n: usize) -> Result<Vec<T>, ByteReaderError> {
        let cursor = self.cursor;
        let res = self.read_n::<T>(n);
        self.cursor = cursor;
        res
    }

    // could be renamed read_sized_vec
    pub fn read_vec<T: ByteReaderResource>(&mut self) -> Result<Vec<T>, ByteReaderError> {
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

pub struct ByteReaderIterator<'a, T: TryFromBytes> {
    buf: &'a mut ByteReader<'a>,
    resource_type: PhantomData<T>,
}

impl<'a, T: ByteReaderResource> Iterator for ByteReaderIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.buf.read::<T>().ok()
    }
}

#[cfg(test)]
use super::transmutable::ByteError;

#[test]
fn test_bytereader() -> Result<(), ByteError> {
    let data = std::fs::read("texture.text")?;
    let mut reader = ByteReader::new(&data, Endianness::default());
    assert_eq!(reader.read::<u16>()?, 1);
    assert_eq!(reader.read::<u16>()?, 0);
    assert_eq!(reader.read::<u32>()?, 0x4000);
    assert_eq!(reader.read::<u32>()?, 0x2B0C);
    assert_eq!(reader.read::<u32>()?, 0);
    assert_eq!(reader.read::<u16>()?, 0x0080);
    assert_eq!(reader.read::<u16>()?, 0x0080);
    assert_eq!(reader.read::<u16>()?, 0x0049);
    reader.seek(0x1c)?;
    assert_eq!(reader.read_n::<u32>(14)?, vec![
        8192,
        10240,
        10752,
        10880,
        10912,
        10920,
        10928,
        10936,
        0,
        0,
        0,
        0,
        0,
        0,
    ]);
    Ok(())
}
