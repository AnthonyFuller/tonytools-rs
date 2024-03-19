use std::io::BufRead;

use crate::{
    util::{
        bytereader::{ByteReader, ByteReaderErrorKind},
        texture::{get_pixel_size, get_scale_factor},
        transmutable::Endianness,
    },
    Version,
};

use super::{
    structs::{Metadata, RawImage, Tony},
    Error, TextureResult,
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

        if let [fs, fl] = buf.read_n::<u32>(2)?[..] {
            [texture.file_size, texture.metadata.flags] = [fs, fl];
        };

        if let [w, h] = buf.read_n::<u16>(2)?[..] {
            [texture.width, texture.height] = [w as u32, h as u32];
        };

        if let Ok(fmt) = buf.read::<u16>()?.try_into() {
            texture.metadata.format = fmt;
        };

        if let [mc, dm] = buf.read_n::<u8>(2)?[..] {
            [
                texture.mips_count,
                texture.default_mip,
            ] = [mc, dm];
        }

        if (buf.read::<u32>()? == 0x4000) && !is_texd {
            let sf = get_scale_factor(texture.width, texture.height);
            texture.width /= sf;
            texture.height /= sf;
        }

        if let Ok(mds) = buf.read_n::<u32>(14)?.as_slice().try_into() {
            texture.mips_datasizes = mds;
        } else {
            return Err(Error::ByteReaderError(
                buf.err(ByteReaderErrorKind::NoBytes),
            ));
        };

        // Skip the duplicated sizes
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