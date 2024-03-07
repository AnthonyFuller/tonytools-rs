use std::{
    array::TryFromSliceError, cmp, convert::Infallible, io::{self, BufRead, Read, Write}, iter, mem::size_of, num::TryFromIntError, vec
};

use crate::{
    hmtextures::{self, Format, Type},
    util::bytereader::{ByteReader, ByteReaderError},
};
use byteordered::ByteOrdered;
use num::{traits::FromBytes, PrimInt};
#[derive(Default)]
struct Texture<'a> {
    pub magic: u16,
    pub metadata: MetaData,
    pub file_size: u32,
    pub width: u16,
    pub height: u16,
    pub mips_count: u8,
    pub default_mip: u8,
    pub dimensions: u8,
    pub atlas_size: u32,
    pub atlas_offset: u32,

    pub mips_datasizes: [u32; 0xE],

    pub pixels: &'a [u8],
}

#[derive(Default, Debug)]
struct MetaData {
    pub r#type: Type,
    pub format: Format,

    pub flags: u32,

    pub interpret_as: u8,
    pub mips_interpol_mode: u8,
}

impl<'a> Texture<'a> {
    pub fn load(&mut self, data: &[u8]) -> Result<(), hmtextures::Error> {
        let mut buf = ByteReader::new(data);
        self.magic = match buf.read::<u16, 2>() {
            Ok(1) => Err(hmtextures::Error::InvalidMagic),
            Ok(n) => Ok(n),
            Err(e) => Err(e.into()),
        }?;

        self.metadata.r#type = match buf.read::<u16, 2>() {
            Ok(n @ 0..=3) => Ok(n.try_into().unwrap()),
            Ok(_) => Err(hmtextures::Error::UnknownType),
            Err(e) => Err(e.into()),
        }?;

        buf.read::<u32, 4>()?; // SKIP TEXD

        if let [fs, fl] = buf.read_n::<u32, 4>(2)?[..] {
            [self.file_size, self.metadata.flags] = [fs, fl];
        };
        if let [w, h] = buf.read_n::<u16, 2>(2)?[..] {
            [self.width, self.height] = [w, h];
        };

        if let Ok(fmt) = buf.read::<u16, 2>()?.try_into() as Result<Format, TryFromIntError> {
            self.metadata.format = fmt;
        };
        if let [mc, dm, ia, dim, mim] = buf.read_n::<u8, 1>(5)?[..] {
            [
                self.mips_count,
                self.default_mip,
                self.metadata.interpret_as,
                self.dimensions,
                self.metadata.mips_interpol_mode,
            ] = [mc, dm, ia, dim, mim];
        }
        if self.dimensions == 0 {
            return Err(hmtextures::Error::InvalidDimensions);
        }

        if let Ok(mds) =
            buf.read_n::<u32, 4>(14)?.as_slice().try_into() as Result<[u32; 14], TryFromSliceError>
        {
            self.mips_datasizes = mds;
        } else {
            return Err(hmtextures::Error::ByteReaderError(ByteReaderError::NoBytes));
        };

        if let [a_s, a_o] = buf.read_n::<u32, 4>(2)?[..] {
            [self.atlas_size, self.atlas_offset] = [a_s, a_o];
        }
        Ok(())
    }
}

pub fn convert(data: &[u8], output_path: &str, metadata_path: &str, swizzle: bool) {
    unimplemented!();
}

#[test]
fn test_2016() -> Result<(), hmtextures::Error> {
    let file = std::fs::read("texture.text")?;
    let mut texture = Texture::default();

    texture.load(file.as_slice())?;

    println!("{:?}", texture.metadata.format);

    Ok(())
}
