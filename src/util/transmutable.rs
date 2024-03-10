//! transmutable.rs
use std::{io, mem::size_of, string::FromUtf8Error};

use num::traits;

use super::{bytereader::ByteReaderError, bytewriter::ByteWriterError};

#[derive(PartialEq, Clone, Copy, Default)]
pub enum Endianness {
    Little,
    Big,
    #[default]
    Native,
}

trait TInt {}
impl !TInt for String {}
impl !TInt for &str {}

impl TInt for usize {}
impl TInt for u8 {}
impl TInt for u16 {}
impl TInt for u32 {}
impl TInt for u64 {}
impl TInt for u128 {}
impl TInt for isize {}
impl TInt for i8 {}
impl TInt for i16 {}
impl TInt for i32 {}
impl TInt for i64 {}
impl TInt for i128 {}

pub trait TryFromBytes: Sized {
    type Bytes;
    type Error = TryFromBytesError;
    fn try_from_bytes(
        bytes: Self::Bytes,
        endianness: Endianness,
    ) -> Result<(Self, usize), Self::Error>;
}

// pub trait FromBytes: Sized {
//     type Bytes;
//     fn from_bytes(bytes: Self::Bytes, endianness: Endianness) -> Self;
// }

// impl<T: FromBytes> TryFromBytes for T {

//     type Bytes = <T as FromBytes>::Bytes;
//     type Error = Infallible;

//     fn try_from_bytes(bytes: Self::Bytes, endianness: Endianness) -> Result<Self, Self::Error> {
//         Ok(T::from_bytes(bytes, endianness))
//     }
// }

pub trait ToBytes: Sized {
    type Bytes;
    fn to_bytes(&self, endianness: Endianness) -> Self::Bytes;
}

// trait Transmutable = ToBytes + FromBytes;
#[derive(Debug)]
pub enum TryFromBytesError {
    StringFromBytes(FromUtf8Error),
    ArrayFromSlice,
}

impl From<FromUtf8Error> for TryFromBytesError {
    fn from(err: FromUtf8Error) -> Self {
        Self::StringFromBytes(err)
    }
}
impl TryFromBytes for String {
    type Bytes = Vec<u8>;
    type Error = TryFromBytesError;

    fn try_from_bytes(bytes: Self::Bytes, _: Endianness) -> Result<(Self, usize), Self::Error> {
        let mut vec: Vec<u8> = Vec::new();
        for byte in bytes {
            if byte == 0x00 {
                break;
            }
            vec.push(byte);
        }
        let res = String::from_utf8(vec)?;
        let len = res.len();
        Ok((res, len + 1))
    }
}

impl<T: traits::FromBytes<Bytes = [u8; size_of::<T>()]> + TInt> TryFromBytes for T
where
    <T as traits::FromBytes>::Bytes: Sized + for<'a> TryFrom<&'a [u8]>,
{
    type Bytes = Vec<u8>;
    type Error = TryFromBytesError;
    fn try_from_bytes(
        bytes: <Self as TryFromBytes>::Bytes,
        endianness: Endianness,
    ) -> Result<(Self, usize), Self::Error> {
        let array_bytes: &[u8; size_of::<T>()] = &bytes.as_slice()[..size_of::<T>()]
            .try_into()
            .or(Err(TryFromBytesError::ArrayFromSlice))?;
        Ok((
            match endianness {
                Endianness::Big => Self::from_be_bytes(array_bytes),
                Endianness::Little => Self::from_le_bytes(array_bytes),
                Endianness::Native => Self::from_ne_bytes(array_bytes),
            },
            size_of::<T>(),
        ))
    }
}

impl<T: traits::ToBytes + TInt> ToBytes for T
where
    <T as traits::ToBytes>::Bytes: Sized + Into<Vec<u8>>,
{
    type Bytes = Vec<u8>;

    fn to_bytes(&self, endianness: Endianness) -> Self::Bytes {
        match endianness {
            Endianness::Big => self.to_be_bytes().into(),
            Endianness::Little => self.to_le_bytes().into(),
            Endianness::Native => self.to_ne_bytes().into(),
        }
    }
}

impl ToBytes for String {
    type Bytes = Vec<u8>;

    fn to_bytes(&self, _: Endianness) -> Self::Bytes {
        String::from(self.to_owned() + "\0").to_owned().into()
    }
}

#[derive(Debug)]
pub enum ByteError {
    ByteReaderError(ByteReaderError),
    ByteWriterError(ByteWriterError),
    IOError(io::Error)
}

impl From<ByteReaderError> for ByteError {
    fn from(err: ByteReaderError) -> Self {
        Self::ByteReaderError(err)
    }
}

impl From<ByteWriterError> for ByteError {
    fn from(err: ByteWriterError) -> Self {
        Self::ByteWriterError(err)
    }
}

impl From<io::Error> for ByteError {
    fn from(err: io::Error) -> Self {
        Self::IOError(err)
    }
}