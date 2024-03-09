use std::{array::TryFromSliceError, io::BufRead};

use crate::{
    hmtextures::{self, Format, Type},
    util::bytereader::{ByteReader, ByteReaderError, Endianness},
};
#[derive(Default, Debug)]
struct Texture {
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

    pub pixels: Vec<u8>,
}

#[derive(Default, Debug)]
struct MetaData {
    pub r#type: Type,
    pub format: Format,

    pub flags: u32,

    pub interpret_as: u8,
    pub mips_interpol_mode: u8,
}

impl Texture {
    pub fn load(data: &[u8]) -> Result<Self, hmtextures::Error> {
        let mut buf = ByteReader::new(data, Endianness::Little);
        let mut texture = Texture::default();

        texture.magic = match buf.read::<u16>() {
            Ok(1) => Ok(1),
            Ok(_) => Err(hmtextures::Error::InvalidMagic),
            Err(e) => Err(e.into()),
        }?;

        texture.metadata.r#type = match buf.read::<u16>() {
            Ok(n @ 0..=3) => n.try_into().map_err(|_| hmtextures::Error::UnknownType),
            Ok(_) => Err(hmtextures::Error::UnknownType),
            Err(e) => Err(e.into()),
        }?;

        buf.consume(4); // SKIP TEXD

        if let [fs, fl] = buf.read_n::<u32>(2)?[..] {
            [texture.file_size, texture.metadata.flags] = [fs, fl];
        };
        if let [w, h] = buf.read_n::<u16>(2)?[..] {
            [texture.width, texture.height] = [w, h];
        };

        if let Ok(fmt) = buf.read::<u16>()?.try_into() {
            texture.metadata.format = fmt;
        };
        if let [mc, dm, ia, dim, mim] = buf.read_n::<u8>(5)?[..] {
            [
                texture.mips_count,
                texture.default_mip,
                texture.metadata.interpret_as,
                texture.dimensions,
                texture.metadata.mips_interpol_mode,
            ] = [mc, dm, ia, dim, mim];
        }
        buf.consume(1);
        if texture.dimensions != 0 {
            return Err(hmtextures::Error::InvalidDimensions);
        }

        if let Ok(mds) =
            buf.read_n::<u32>(14)?.as_slice().try_into() as Result<[u32; 14], TryFromSliceError>
        {
            texture.mips_datasizes = mds;
        } else {
            return Err(hmtextures::Error::ByteReaderError(ByteReaderError::NoBytes));
        };

        if let [a_s, a_o] = buf.read_n::<u32>(2)?[..] {
            [texture.atlas_size, texture.atlas_offset] = [a_s, a_o];
        }
        texture.pixels = buf.fill_buf()?.to_vec();
        Ok(texture)
    }
}

pub fn convert(data: &[u8], output_path: &str, metadata_path: &str, swizzle: bool) {
    unimplemented!();
}

#[test]
fn test_2016() -> Result<(), hmtextures::Error> {
    let file = std::fs::read("texture.text")?;
    let texture = Texture::load(file.as_slice());

    println!("{:#?}", texture);
    Ok(())
}
