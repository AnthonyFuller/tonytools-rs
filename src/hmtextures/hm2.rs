#![allow(dead_code)]
use bitchomp::{ByteReader, ChompFlatten, Endianness};
use std::io::BufRead;

use crate::{
    util::texture::{get_pixel_size, get_scale_factor},
    Version,
};

use super::{
    structs::{Metadata, RawImage},
    Error,
};

#[derive(Default, Debug)]
struct Texture {
    pub magic: u16,
    pub metadata: Metadata,
    pub file_size: u32,
    pub width: u32,
    pub height: u32,
    pub mips_count: u8,
    pub default_mip: u8,
    pub atlas_size: u32,
    pub atlas_offset: u32,

    pub mips_datasizes: [u32; 0xE],
    pub pixels: Vec<u8>,
}

impl Texture {
    pub fn load(data: &[u8], is_texd: bool) -> Result<Self, Error> {
        let mut buf = ByteReader::new(data, Endianness::Little);
        let mut texture = Texture::default();
        texture.metadata.version = Version::H2;

        if buf.read::<u16>()?.inner() != 1 {
            return Err(Error::InvalidMagic);
        }

        let r#type = buf.read::<u16>()?.inner();
        if r#type > 3 {
            return Err(Error::UnknownType);
        }
        texture.metadata.r#type = r#type.try_into().unwrap();

        // Skip file size
        buf.consume(0x4);

        texture.metadata.flags = buf.read()?.inner();

        if let [w, h] = buf.read_n::<u16>(2)?.flatten()[..] {
            [texture.width, texture.height] = [w as u32, h as u32];
        };

        if let Ok(fmt) = buf.read::<u16>()?.inner().try_into() {
            texture.metadata.format = fmt;
        };

        // Skip mip count and default mip
        buf.consume(0x2);

        if (buf.read::<u32>()?.inner() == 0x4000) && !is_texd {
            let sf = get_scale_factor(texture.width, texture.height);
            texture.width /= sf;
            texture.height /= sf;
        }

        // Skip the mip sizes
        buf.consume(0xE * 4 * 2);

        if let [a_s, a_o] = buf.read_n::<u32>(2)?.flatten()[..] {
            [texture.atlas_size, texture.atlas_offset] = [a_s, a_o];
        }

        if texture.atlas_size != 0 {
            return Err(Error::AtlasNotSupported);
        }

        texture.pixels = buf.cursor.to_vec();
        Ok(texture)
    }
}

impl From<Texture> for RawImage {
    fn from(val: Texture) -> Self {
        RawImage {
            width: val.width,
            height: val.width,
            pixels: val.pixels
                [..get_pixel_size(val.metadata.format, val.width, val.height, 0) as usize]
                .to_vec(),
            metadata: val.metadata,
        }
    }
}
