#![allow(dead_code)]
use std::cmp::{max, min};

use num::ToPrimitive;

use crate::hmtextures::Format;

pub fn bits_per_pixel(format: Format) -> u32 {
    match format {
        Format::R16G16B16A16 => 64,
        Format::R8G8B8A8 => 32,
        Format::R8G8 => 16,
        Format::A8 | Format::DXT5 | Format::BC5 | Format::BC7 => 8,
        Format::DXT1 | Format::BC4 => 4,
        _ => 0,
    }
}

pub fn get_scale_factor(width: u32, height: u32) -> u32 {
    let area = (width * height) as f32;
    return if (1 << 15) as f32 <= area && area <= (1 << 24) as f32 {
        2_u32.pow(((area.log2() - 13.0) / 2.0).floor().to_u32().unwrap())
    } else {
        1
    };
}

pub fn max_mip_count(width: u32, _: u32) -> u32 {
    min(1 + (width as f32).log2().floor() as u32, 0xE)
}

pub fn pixel_block_size(format: Format) -> u32 {
    match format {
        Format::DXT1 | Format::DXT5 | Format::BC4 | Format::BC5 | Format::BC7 => 4,
        _ => 1,
    }
}

pub fn compute_pitch(format: Format, width: u32, height: u32) -> (u32, u32) {
    let pitch;
    let slice;

    match format {
        Format::DXT1 | Format::BC4 => {
            let nbw = max(1, (width + 3) / 4);
            let nbh = max(1, (height + 3) / 4);
            pitch = nbw * 8;
            slice = pitch * nbh;
        }
        Format::DXT5 | Format::BC5 | Format::BC7 => {
            let nbw = max(1, (width + 3) / 4);
            let nbh = max(1, (height + 3) / 4);
            pitch = nbw * 16;
            slice = pitch * nbh;
        }
        _ => {
            let bpp = bits_per_pixel(format);
            pitch = (width * bpp + 7) / 8;
            slice = pitch * height;
        }
    }

    (pitch, slice)
}

pub fn get_pixel_size(format: Format, mut width: u32, mut height: u32, mut mip_level: u8) -> u32 {
    let max_mip = max_mip_count(width, height) as u8;
    if mip_level > max_mip {
        mip_level = max_mip;
    }

    height >>= mip_level;
    width >>= mip_level;

    let (_, slice) = compute_pitch(format, width, height);

    slice
}

pub fn get_total_size(format: Format, width: u32, height: u32, mip_levels: u8) -> u32 {
    let mut size: u32 = 0;

    for i in 0..mip_levels {
        size += get_pixel_size(format, width, height, i);
    }

    size
}
