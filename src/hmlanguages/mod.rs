use std::{num::ParseIntError, string::FromUtf8Error};

use bitchomp::{ByteReaderError, ByteWriterError};
use thiserror::Error;

pub mod clng;
pub mod ditl;
pub mod dlge;
pub mod hashlist;
pub mod locr;
pub mod rtlv;

#[derive(Debug, Error)]
pub enum LangError {
    #[error("invalid language map")]
    InvalidLanguageMap,

    #[error("did not reach end-of-file")]
    DidNotReachEOF,

    #[error("json error: {0}")]
    JsonError(serde_json::Error),

    #[error("unsupported version")]
    UnsupportedVersion,

    #[error("byter reader error: {0}")]
    ByteReaderError(ByteReaderError),

    #[error("byte writer error: {0}")]
    ByteWriterError(ByteWriterError),

    #[error("utf-8 error: {0}")]
    Utf8Error(FromUtf8Error),

    #[error("invalid container: {0}")]
    InvalidContainer(u8),

    #[error("invalid reference: {0}")]
    InvalidReference(u8),

    #[error("parse int error: {0}")]
    ParseIntError(ParseIntError),

    #[error("invalid input")]
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

#[derive(Debug)]
pub struct Rebuilt {
    pub file: Vec<u8>,
    pub meta: String,
}
