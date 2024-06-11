#![allow(dead_code)]
use bitchomp::{ByteReader, Endianness, ChompFlatten};
use std::io::BufRead;

use crate::{
    util::texture::{get_pixel_size, get_total_size},
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

    pub texture_sizes: [u32; 0xE],
    pub pixels: Vec<u8>,
}

impl Texture {
    pub fn load(data: &[u8], texd: Option<&[u8]>) -> Result<Self, Error> {
        let mut buf = ByteReader::new(data, Endianness::Little);
        let mut texture = Texture::default();
        texture.metadata.version = Version::H3;

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

        texture.metadata.interpret_as = buf.read()?.inner();

        buf.consume(0x1);

        texture.metadata.interpol_mode = buf.read()?.inner();

        let texture_sizes = buf.read_n::<u32>(0xE)?.flatten();
        let compressed_sizes = buf.read_n::<u32>(0xE)?.flatten();

        if let [a_s, a_o] = buf.read_n::<u32>(2)?.flatten()[..] {
            [texture.atlas_size, texture.atlas_offset] = [a_s, a_o];
        }

        if texture.atlas_size != 0 {
            return Err(Error::AtlasNotSupported);
        }

        // Skip scaling data
        buf.consume(0x01);

        let width_sf = match buf.read::<u8>()?.inner() {
            0 => 0,
            n => 2 << (n - 1),
        };
        let height_sf = match buf.read::<u8>()?.inner() {
            0 => 0,
            n => 2 << (n - 1),
        };
        let text_mip_count: u8 = buf.read()?.inner();

        // Skip padding
        buf.consume(0x04);

        texture.pixels = buf.cursor.to_vec();

        // We only return the highest quality texture as the pixels
        texture.pixels = if let Some(texd) = texd {
            lz4_flex::block::decompress(
                &texd[..compressed_sizes[0] as usize],
                texture_sizes[0] as usize,
            )
            .unwrap()
        } else if texture_sizes[0] != compressed_sizes[0] {
            if width_sf != 0 && height_sf != 0 {
                texture.width /= width_sf;
                texture.height /= height_sf;
            }

            let text_size = get_total_size(
                texture.metadata.format,
                texture.width,
                texture.height,
                text_mip_count,
            );

            // We decompress the entire pixels object here as it's compressed
            // like that.
            lz4_flex::block::decompress(&texture.pixels, text_size as usize).unwrap()
                [..get_pixel_size(texture.metadata.format, texture.width, texture.height, 0)
                    as usize]
                .to_vec()
        } else {
            texture.pixels[..texture_sizes[0] as usize].to_vec()
        };

        Ok(texture)
    }
}

impl From<Texture> for RawImage {
    fn from(val: Texture) -> Self {
        RawImage {
            width: val.width,
            height: val.width,
            pixels: val.pixels,
            metadata: val.metadata,
        }
    }
}
