#![allow(dead_code)]
use crate::Version;
use bitchomp::{ByteWriter, Endianness};
use texture2ddecoder::{decode_bc1, decode_bc3, decode_bc4, decode_bc5, decode_bc7};

use super::{ColourType, Format, Type};

#[derive(Default, Debug, Clone)]
pub struct Metadata {
    pub version: Version,
    pub r#type: Type,
    pub format: Format,
    pub flags: u32,
    pub interpret_as: u8,
    pub interpol_mode: u16,
}

impl Metadata {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = ByteWriter::new(Endianness::Little);

        buf.append(self.version as u8);
        buf.append(self.r#type as u8);
        buf.append(self.format as u16);
        buf.append(self.flags);
        buf.append(self.interpret_as);

        buf.buf()
    }
}

#[derive(Debug, Clone)]
pub struct RawImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub metadata: Metadata,
}

pub struct Tony {
    pub magic: u32,
    pub colour_type: ColourType,
    pub width: u32,
    pub height: u32,
    pub decompressed_size: u64,
    pub compressed_size: u64,
    pub data: Vec<u8>,
    pub metadata: Metadata,
}

impl Tony {
    pub fn new(
        colour_type: ColourType,
        width: u32,
        height: u32,
        data: Vec<u8>,
        metadata: Metadata,
    ) -> Self {
        let compressed = lz4_flex::block::compress(&data);

        Self {
            magic: 0x594E4F54, // TONY
            colour_type,
            width,
            height,
            decompressed_size: data.len() as u64,
            compressed_size: compressed.len() as u64,
            data: compressed,
            metadata,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = ByteWriter::new(Endianness::Little);

        buf.append(self.magic);
        buf.append(self.colour_type as u8);
        buf.append(self.width);
        buf.append(self.height);
        buf.append(self.decompressed_size);
        buf.append(self.compressed_size);
        buf.append_vec(self.data.clone());
        buf.append_vec(self.metadata.serialize());

        buf.buf()
    }
}

fn get_image_pixels(img: RawImage) -> (ColourType, Vec<u8>) {
    let mut pixels = vec![0_u32; (img.width * img.height) as usize];
    let mut data: Vec<u8> = Vec::new();
    let mut fix_channel = false;

    let mut colour = ColourType::Rgba8;
    match img.metadata.format {
        Format::R16G16B16A16 => {
            colour = ColourType::Rgba16;
            data = img.pixels.clone();
        }
        Format::R8G8B8A8 => {
            data = img.pixels.clone();
        }
        Format::R8G8 => {
            colour = ColourType::Rgb8;
            data = img
                .pixels
                .chunks_exact(2)
                .flat_map(|e| [e[0], e[1], 0xFF])
                .collect();
        }
        Format::A8 => {
            colour = ColourType::L8;
            data = img.pixels.clone();
        }
        Format::DXT1 => {
            decode_bc1(
                &img.pixels,
                img.width as usize,
                img.height as usize,
                pixels.as_mut_slice(),
            )
            .unwrap();
        }
        Format::DXT5 => {
            decode_bc3(
                &img.pixels,
                img.width as usize,
                img.height as usize,
                pixels.as_mut_slice(),
            )
            .unwrap();
        }
        Format::BC4 => {
            colour = ColourType::L8;

            decode_bc4(
                &img.pixels,
                img.width as usize,
                img.height as usize,
                pixels.as_mut_slice(),
            )
            .unwrap();
        }
        Format::BC5 => {
            fix_channel = true;

            decode_bc5(
                &img.pixels,
                img.width as usize,
                img.height as usize,
                pixels.as_mut_slice(),
            )
            .unwrap();
        }
        Format::BC7 => {
            decode_bc7(
                &img.pixels,
                img.width as usize,
                img.height as usize,
                pixels.as_mut_slice(),
            )
            .unwrap();
        }
        _ => {}
    }

    match img.metadata.format {
        Format::R16G16B16A16 | Format::R8G8B8A8 | Format::R8G8 | Format::A8 => {}
        _ => {
            data = pixels
                .iter()
                .flat_map(|x| {
                    let v = x.to_le_bytes();
                    let b = if fix_channel { 0xFF } else { v[0] };
                    [v[2], v[1], b, v[3]]
                })
                .collect();
        }
    }

    (colour, data)
}

impl From<RawImage> for Tony {
    fn from(img: RawImage) -> Self {
        let (colour, data) = get_image_pixels(img.clone());

        Tony::new(colour, img.width, img.height, data, img.metadata)
    }
}
