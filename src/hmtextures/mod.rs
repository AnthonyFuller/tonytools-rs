use std::{default, io, num::TryFromIntError};

use crate::util::bytereader::ByteReaderError;

pub mod hm2016;

#[derive(Debug)]
enum Error {
    InvalidMagic,
    InvalidDimensions,
    UnknownType,
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

#[derive(Default, Debug)]
enum Type {
    Colour,
    Normal,
    Height,
    CompoundNormal,
    Billboard,
    #[default]
    Default,
}
impl From<Type> for u16 {
    fn from(r#type: Type) -> Self {
        r#type as u16
    }
}
impl TryFrom<u16> for Type {
    type Error = TryFromIntError;
    fn try_from(n: u16) -> Result<Type, TryFromIntError> {
        n.try_into()
    }
}
#[derive(Default, Debug)]
enum Format {
    R16G16B16A16 = 0x0A,
    R8G8B8A8 = 0x1C,
    R8G8 = 0x34, //Normals. very rarely used. Legacy? Only one such tex in chunk0;
    A8 = 0x42,   //8-bit grayscale uncompressed. Not used on models?? Overlays
    DXT1 = 0x49, //Color maps, 1-bit alpha (mask). Many uses, color, normal, spec, rough maps on models and decals. Also used as masks.
    DXT3 = 0x4C,
    DXT5 = 0x4F, //Packed color, full alpha. Similar use as DXT5.
    BC4 = 0x52,  //8-bit grayscale. Few or no direct uses on models?
    BC5 = 0x55,  //2-channel normal maps
    BC7 = 0x5A,  //high res color + full alpha. Used for pretty much everything...
    #[default]
    Default
}
impl From<Format> for u16 {
    fn from(format: Format) -> Self {
        format as u16
    }
}
impl TryFrom<u16> for Format {
    type Error = TryFromIntError;
    fn try_from(n: u16) -> Result<Format, TryFromIntError> {
        n.try_into()
    }
}
struct BuiltTexture<'a> {
    pub width: u16,
    pub height: u16,
    pub mips_count: u8,

    pub mips_sizes: [u32; 0xE],
    pub compressed_sizes: [u32; 0xE],

    pub pixels: &'a [u8],
    pub compressed_pixels: &'a [u8],
}
