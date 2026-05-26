use byteorder::LE;
use extended_tea::XTEA;
use once_cell::sync::Lazy;

use crate::{LangError, Version, hmlanguages::LangResult};

static XTEA_WOA: Lazy<XTEA> =
    Lazy::new(|| XTEA::new(&[0x53527737u32, 0x7506499Eu32, 0xBD39AEE3u32, 0xA59E7268u32]));

static XTEA_KNT: Lazy<XTEA> =
    Lazy::new(|| XTEA::new(&[0x68AC3361u32, 0x562B4AA0u32, 0xB9F2771Fu32, 0x28EB3CE7u32]));

pub fn xtea_decrypt(version: Version, data: Vec<u8>) -> LangResult<String> {
    let mut out_data = data.clone();

    match version {
        Version::KNT => XTEA_KNT.decipher_u8slice::<LE>(&data, &mut out_data),
        Version::H2016 | Version::H2 | Version::H3 => XTEA_WOA.decipher_u8slice::<LE>(&data, &mut out_data),
        _ => return Err(LangError::UnsupportedVersion),
    };

    Ok(String::from_utf8(out_data)?
        .trim_matches(char::from(0))
        .to_string())
}

pub fn xtea_encrypt(version: Version, str: &str) -> LangResult<Vec<u8>> {
    let mut str = str.as_bytes().to_vec();
    if !str.len().is_multiple_of(8) {
        str.extend(vec![0; 8 - (str.len() % 8)]);
    }

    let mut out_data = vec![0; str.len()];

    match version {
        Version::KNT => XTEA_KNT.encipher_u8slice::<LE>(&str, &mut out_data),
        Version::H2016 | Version::H2 | Version::H3 => XTEA_WOA.encipher_u8slice::<LE>(&str, &mut out_data),
        _ => return Err(LangError::UnsupportedVersion),
    };

    Ok(out_data)
}

pub fn symmetric_encrypt(data: Vec<u8>) -> Vec<u8> {
    let mut data = data.clone();
    for char in data.as_mut_slice() {
        let value = *char;
        *char ^= 226;
        *char = (value & 0x81)
            | (value & 2) << 1
            | (value & 4) << 2
            | (value & 8) << 3
            | (value & 0x10) >> 3
            | (value & 0x20) >> 2
            | (value & 0x40) >> 1;
    }

    data
}

pub fn symmetric_decrypt(mut data: Vec<u8>) -> LangResult<String> {
    for char in data.as_mut_slice() {
        let value = *char;
        *char = (value & 1)
            | (value & 2) << 3
            | (value & 4) >> 1
            | (value & 8) << 2
            | (value & 16) >> 2
            | (value & 32) << 1
            | (value & 64) >> 3
            | (value & 128);
        *char ^= 226;
    }

    Ok(String::from_utf8(data)?)
}
