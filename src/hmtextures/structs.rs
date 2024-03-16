use crate::{hmlanguages::LangResult, util::{bytewriter::ByteWriter, transmutable::Endianness}, Version};
use texture2ddecoder::{decode_bc1, decode_bc3, decode_bc4, decode_bc5, decode_bc7};

use super::{ColourType, Format, Type};

#[derive(Default, Debug)]
pub struct Metadata {
    pub version: Version,
    pub r#type: Type,
    pub format: Format,
    pub flags: u32,
    pub interpret_as: u8,
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

#[derive(Debug)]
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
    pub decompressed_size: u32,
    pub compressed_size: u32,
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
            magic: 0x544F4E59, // TONY
            colour_type,
            width,
            height,
            decompressed_size: data.len() as u32,
            compressed_size: compressed.len() as u32,
            data: compressed,
            metadata,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = ByteWriter::new(Endianness::Little);

        buf.append(self.magic);
        //buf.append(self.colour_type);
        buf.append(self.width);
        buf.append(self.height);
        buf.append(self.decompressed_size);
        buf.append(self.compressed_size);
        buf.write_vec(self.data.clone());
        buf.write_vec(self.metadata.serialize());

        buf.buf()
    }
}

impl Into<Tony> for RawImage {
    fn into(self) -> Tony {
        let mut pixels = vec![0_u32; (self.width * self.height) as usize];
        let mut data: Vec<u8> = Vec::new();
        let mut fix_channel = false;

        let mut colour = ColourType::Rgba8;
        match self.metadata.format {
            Format::R16G16B16A16 => {
                colour = ColourType::Rgba16;
                data = self.pixels.clone();
            }
            Format::R8G8B8A8 => {
                data = self.pixels.clone();
            }
            Format::R8G8 => {
                colour = ColourType::Rgb8;
                data = self
                    .pixels
                    .chunks_exact(2)
                    .flat_map(|e| [e[0], e[1], 0xFF])
                    .collect();
            }
            Format::A8 => {
                colour = ColourType::L8;
                data = self.pixels.clone();
            }
            Format::DXT1 => {
                decode_bc1(
                    &self.pixels,
                    self.width as usize,
                    self.height as usize,
                    pixels.as_mut_slice(),
                )
                .unwrap();
            }
            Format::DXT5 => {
                decode_bc3(
                    &self.pixels,
                    self.width as usize,
                    self.height as usize,
                    pixels.as_mut_slice(),
                )
                .unwrap();
            }
            Format::BC4 => {
                colour = ColourType::L8;

                decode_bc4(
                    &self.pixels,
                    self.width as usize,
                    self.height as usize,
                    pixels.as_mut_slice(),
                )
                .unwrap();
            }
            Format::BC5 => {
                fix_channel = true;

                decode_bc5(
                    &self.pixels,
                    self.width as usize,
                    self.height as usize,
                    pixels.as_mut_slice(),
                )
                .unwrap();
            }
            Format::BC7 => {
                decode_bc7(
                    &self.pixels,
                    self.width as usize,
                    self.height as usize,
                    pixels.as_mut_slice(),
                )
                .unwrap();
            }
            _ => {}
        }

        match self.metadata.format {
            Format::R16G16B16A16 | Format::R8G8B8A8 | Format::R8G8 | Format::A8 => {}
            _ => {
                data = pixels
                    .iter()
                    .flat_map(|x| {
                        let v = x.to_le_bytes();
                        let b = if fix_channel { 0xFF } else { v[0] };
                        [v[2], v[1], b, v[3]]
                    })
                    .collect::<Vec<u8>>();
            }
        }

        Tony::new(colour, self.width, self.height, data, self.metadata)
    }
}
