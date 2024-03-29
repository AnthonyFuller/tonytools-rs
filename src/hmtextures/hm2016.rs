use bitchomp::{ByteReader, Endianness};
use std::io::BufRead;

use crate::{
    hmtextures::Error,
    util::texture::{get_pixel_size, get_scale_factor},
    Version,
};

use super::structs::{Metadata, RawImage};

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

        match buf.read::<u16>() {
            Ok(1) => Ok(1),
            Ok(_) => Err(Error::InvalidMagic),
            Err(e) => Err(e.into()),
        }?;

        texture.metadata.r#type = match buf.read::<u16>() {
            Ok(n @ 0..=3) => n.try_into().map_err(|_| Error::UnknownType),
            Ok(_) => Err(Error::UnknownType),
            Err(e) => Err(e.into()),
        }?;

        let is_texd = (buf.read::<u32>()? == 0x4000) && is_texd;

        // Skip file size
        buf.consume(0x4);

        texture.metadata.flags = buf.read()?;

        if let [w, h] = buf.read_n::<u16>(2)?[..] {
            [texture.width, texture.height] = [w as u32, h as u32];
        };

        if !is_texd {
            let sf = get_scale_factor(texture.width, texture.height);
            texture.width /= sf;
            texture.height /= sf;
        }

        if let Ok(fmt) = buf.read::<u16>()?.try_into() {
            texture.metadata.format = fmt;
        };

        // Skip mip count + default mip
        buf.consume(0x2);

        texture.metadata.interpret_as = buf.read()?;

        if buf.read::<u8>()? != 0 {
            return Err(Error::InvalidDimensions);
        }

        texture.metadata.interpol_mode = buf.read::<u8>()? as u16;

        // Skip mip sizes
        buf.consume(0xE * 4);

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
            pixels: self.pixels
                [..get_pixel_size(self.metadata.format, self.width, self.height, 0) as usize]
                .to_vec(),
            metadata: self.metadata,
        }
    }
}
