use byteorder::LE;
use extended_tea::XTEA;
use once_cell::sync::Lazy;

use crate::hmlanguages::LangResult;

const XTEA: Lazy<XTEA> =
    Lazy::new(|| XTEA::new(&[0x53527737u32, 0x7506499Eu32, 0xBD39AEE3u32, 0xA59E7268u32]));

pub fn xtea_decrypt(data: Vec<u8>) -> LangResult<String> {
    let mut out_data = data.clone();

    XTEA.decipher_u8slice::<LE>(&data, &mut out_data);
    Ok(String::from_utf8(out_data)?
        .trim_matches(char::from(0))
        .to_string())
}

pub fn xtea_encrypt(str: &str) -> Vec<u8> {
    let mut str = str.as_bytes().to_vec();
    if str.len() % 8 != 0 {
        str.extend(vec![0; 8 - (str.len() % 8)]);
    }

    let mut out_data = vec![0; str.len()];

    XTEA.encipher_u8slice::<LE>(&str, &mut out_data);

    out_data
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
