use std::{convert::Infallible, io};

use bitchomp::bytereader::ByteReaderError;

pub mod hm2;
pub mod hm2016;
pub mod hm3;
pub mod structs;

pub type TextureResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidMagic,
    InvalidDimensions,
    UnknownType,
    UnknownFormat,
    AtlasNotSupported,
    ByteReaderError(ByteReaderError),
    IOError(io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}

impl From<ByteReaderError> for Error {
    fn from(err: ByteReaderError) -> Self {
        Error::ByteReaderError(err)
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Type {
    Colour,
    Normal,
    Height,
    CompoundNormal,
    Billboard,
    #[default]
    Unknown,
}

impl From<Type> for u16 {
    fn from(r#type: Type) -> Self {
        r#type as u16
    }
}

impl TryFrom<u16> for Type {
    type Error = self::Error;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Ok([
            Self::Colour,
            Self::Normal,
            Self::Height,
            Self::CompoundNormal,
            Self::Billboard,
            Self::Unknown,
        ][(value.try_into() as Result<usize, Infallible>).map_err(|_| self::Error::UnknownType)?])
    }
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub enum Format {
    #[default]
    Unknown = 0,
    R16G16B16A16 = 0x0A,
    R8G8B8A8 = 0x1C,
    R8G8 = 0x34, //Normals. very rarely used. Legacy? Only one such tex in chunk0;
    A8 = 0x42,   //8-bit grayscale uncompressed. Not used on models?? Overlays
    DXT1 = 0x49, //Color maps, 1-bit alpha (mask). Many uses, color, normal, spec, rough maps on models and decals. Also used as masks.
    DXT5 = 0x4F, //Packed color, full alpha. Similar use as DXT5.
    BC4 = 0x52,  //8-bit grayscale. Few or no direct uses on models?
    BC5 = 0x55,  //2-channel normal maps
    BC7 = 0x5A,  //high res color + full alpha. Used for pretty much everything...
}

impl From<Format> for u16 {
    fn from(format: Format) -> Self {
        format as u16
    }
}

impl TryFrom<u16> for Format {
    type Error = self::Error;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x0A => Ok(Self::R16G16B16A16),
            0x1C => Ok(Self::R8G8B8A8),
            0x34 => Ok(Self::R8G8),
            0x42 => Ok(Self::A8),
            0x49 => Ok(Self::DXT1),
            0x4F => Ok(Self::DXT5),
            0x52 => Ok(Self::BC4),
            0x55 => Ok(Self::BC5),
            0x5A => Ok(Self::BC7),
            _ => Err(self::Error::UnknownFormat),
        }
    }
}

// Cut down version of the one in the image crate.
#[derive(Copy, Clone)]
pub enum ColourType {
    L8,
    Rgb8,
    Rgba8,
    Rgba16,
}
