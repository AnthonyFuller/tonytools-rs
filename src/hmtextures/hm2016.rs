use std::{io::BufRead};

use crate::{
    hmtextures::Error,
    util::{
        bytereader::{ByteReader, ByteReaderErrorKind},
        transmutable::Endianness,
    },
    Version,
};

use super::{structs::{Metadata, RawImage, Tony}, TextureResult};
#[derive(Default, Debug)]
struct Texture {
    pub magic: u16,
    pub metadata: Metadata,
    pub file_size: u32,
    pub width: u32,
    pub height: u32,
    pub mips_count: u8,
    pub default_mip: u8,
    pub dimensions: u8,
    pub mips_interpol_mode: u8,
    pub atlas_size: u32,
    pub atlas_offset: u32,

    pub mips_datasizes: [u32; 0xE],
    pub pixels: Vec<u8>,
}

impl Texture {
    pub fn load(data: &[u8], is_texd: bool) -> Result<Self, Error> {
        let mut buf = ByteReader::new(data, Endianness::Little);
        let mut texture = Texture::default();
        texture.metadata.version = Version::H2016;

        texture.magic = match buf.read::<u16>() {
            Ok(1) => Ok(1),
            Ok(_) => Err(Error::InvalidMagic),
            Err(e) => Err(e.into()),
        }?;

        texture.metadata.r#type = match buf.read::<u16>() {
            Ok(n @ 0..=3) => n.try_into().map_err(|_| Error::UnknownType),
            Ok(_) => Err(Error::UnknownType),
            Err(e) => Err(e.into()),
        }?;

        buf.consume(4); // SKIP TEXD

        if let [fs, fl] = buf.read_n::<u32>(2)?[..] {
            [texture.file_size, texture.metadata.flags] = [fs, fl];
        };
        if let [w, h] = buf.read_n::<u16>(2)?[..] {
            [texture.width, texture.height] = [w as u32, h as u32];
        };

        if !is_texd {
            texture.width /= 4;
            texture.height /= 4;
        }

        if let Ok(fmt) = buf.read::<u16>()?.try_into() {
            texture.metadata.format = fmt;
        };
        if let [mc, dm, ia, dim, mim] = buf.read_n::<u8>(5)?[..] {
            [
                texture.mips_count,
                texture.default_mip,
                texture.metadata.interpret_as,
                texture.dimensions,
                texture.mips_interpol_mode,
            ] = [mc, dm, ia, dim, mim];
        }
        buf.consume(1);
        if texture.dimensions != 0 {
            return Err(Error::InvalidDimensions);
        }

        if let Ok(mds) = buf.read_n::<u32>(14)?.as_slice().try_into() {
            texture.mips_datasizes = mds;
        } else {
            return Err(Error::ByteReaderError(
                buf.err(ByteReaderErrorKind::NoBytes),
            ));
        };

        if let [a_s, a_o] = buf.read_n::<u32>(2)?[..] {
            [texture.atlas_size, texture.atlas_offset] = [a_s, a_o];
        }
        if texture.atlas_size != 0 {
            return Err(Error::AtlasNotSupported);
        }

        texture.pixels = buf.cursor.to_vec();
        Ok(texture)
    }
}

impl Into<RawImage> for Texture {
    fn into(self) -> RawImage {
        RawImage {
            width: self.width,
            height: self.width,
            pixels: self.pixels[..(self.mips_datasizes[0] as usize)].to_vec(),
            metadata: self.metadata,
        }
    }
}

#[test]
fn test_hm2016() -> TextureResult<()> {
    let file = std::fs::read("texture-a8.texd")?;
    let texture = Texture::load(file.as_slice(), true)?;
    let tony: Tony = Into::<RawImage>::into(texture).into();

    Ok(())
}
