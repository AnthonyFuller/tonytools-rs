use std::{num::ParseIntError, string::FromUtf8Error};

use crate::util::{bytereader::ByteReaderError, bytewriter::ByteWriterError};

pub mod clng;
pub mod ditl;
pub mod dlge;
pub mod hashlist;
pub mod locr;

#[derive(Debug)]
pub enum LangError {
    InvalidLanguageMap,
    DidNotReachEOF,
    JsonError(serde_json::Error),
    UnsupportedVersion,
    ByteReaderError(ByteReaderError),
    ByteWriterError(ByteWriterError),
    Utf8Error(FromUtf8Error),
    InvalidContainer(u8),
    InvalidReference(u8),
    ParseIntError(ParseIntError),
    InvalidInput,
}

impl From<ByteReaderError> for LangError {
    fn from(err: ByteReaderError) -> Self {
        LangError::ByteReaderError(err)
    }
}

impl From<ByteWriterError> for LangError {
    fn from(err: ByteWriterError) -> Self {
        LangError::ByteWriterError(err)
    }
}

impl From<ParseIntError> for LangError {
    fn from(err: ParseIntError) -> Self {
        LangError::ParseIntError(err)
    }
}

impl From<serde_json::Error> for LangError {
    fn from(err: serde_json::Error) -> Self {
        LangError::JsonError(err)
    }
}

impl From<FromUtf8Error> for LangError {
    fn from(err: FromUtf8Error) -> Self {
        LangError::Utf8Error(err)
    }
}

pub type LangResult<T> = Result<T, LangError>;

pub struct Rebuilt {
    pub file: Vec<u8>,
    pub meta: String,
}
