use super::super::vec_of_strings;
use super::Rebuilt;
use super::{hashlist::HashList, LangError, LangResult};
use crate::util::bytereader::{ByteReader, Endianness};
use crate::util::rpkg;
use crate::Version;
use byteorder::LE;
use extended_tea::XTEA;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const XTEA: Lazy<XTEA> =
    Lazy::new(|| XTEA::new(&[0x53527737u32, 0x7506499Eu32, 0xBD39AEE3u32, 0xA59E7268u32]));

fn symmetric_decrypt(mut data: Vec<u8>) -> LangResult<String> {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct LocrJson {
    #[serde(rename = "$schema")]
    schema: String,
    hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symmetric: Option<bool>,
    languages: serde_json::Value,
}

pub struct LOCR {
    hashlist: HashList,
    version: Version,
    lang_map: Vec<String>,
    symmetric: bool,
}

impl LOCR {
    pub fn new(
        hashlist: HashList,
        version: Version,
        lang_map: Option<String>,
        symmetric: bool,
    ) -> LangResult<Self> {
        let lang_map = if lang_map.is_none() {
            match version {
                Version::H2016 | Version::H2 => vec_of_strings![
                    "xx", "en", "fr", "it", "de", "es", "ru", "mx", "br", "pl", "cn", "jp", "tc"
                ],
                Version::H3 => {
                    vec_of_strings!["xx", "en", "fr", "it", "de", "es", "ru", "cn", "tc", "jp"]
                }
                _ => return Err(LangError::UnsupportedVersion),
            }
        } else {
            lang_map.unwrap().split(",").map(|s| s.to_string()).collect()
        };

        Ok(LOCR {
            hashlist,
            version,
            lang_map,
            symmetric,
        })
    }

    pub fn convert(&self, data: &[u8], meta_json: String) -> LangResult<LocrJson> {
        let mut buf = ByteReader::new(data, Endianness::Little);

        let is_locr_v2 = if self.version != Version::H2016 {
            buf.read::<u8>()?;
            true
        } else {
            false
        };

        let mut j: LocrJson = serde_json::from_str(
            r#"{
            "$schema": "https://tonytools.win/schemas/locr.schema.json",
            "hash": "",
            "symmetric": true,
            "languages": {}
        }"#,
        )
        .expect("Something has gone horribly wrong.");

        if !self.symmetric || self.version != Version::H2016 {
            j.symmetric = None;
        }

        let cursor = buf.cursor();
        let num_languages = ((buf.read::<u32>()? - is_locr_v2 as u32) / 4) as usize;
        if num_languages > self.lang_map.len() {
            return Err(LangError::InvalidLanguageMap);
        }
        buf.seek(cursor)?;

        let offsets = buf.read_n::<u32>(num_languages as usize)?;
        for i in 0..num_languages {
            let language = self.lang_map.get(i).expect("Something went wrong");
            j.languages[language] = Value::Object(Map::new());

            if offsets[i] == u32::MAX {
                continue;
            }
            buf.seek(offsets[i] as usize)?;

            for _ in 0..buf.read::<u32>()? {
                let hash_num = buf.read::<u32>()?;
                let hex: String = format!("{:08X}", hash_num);
                let hash = self.hashlist.lines.get_by_left(&hash_num).unwrap_or(&hex);
                let str_data = buf.read_vec::<u8>()?;
                let mut out_data = str_data.clone();
                buf.seek(buf.cursor() + 1)?; // Skip null terminator

                j.languages[language][hash] = match self.symmetric {
                    true => symmetric_decrypt(str_data)?.into(),
                    false => {
                        XTEA.decipher_u8slice::<LE>(&str_data, &mut out_data);
                        String::from_utf8(out_data)?
                            .trim_matches(char::from(0))
                            .into()
                    }
                }
            }
        }

        let meta: rpkg::ResourceMeta = serde_json::from_str(meta_json.as_str())?;
        j.hash = meta.hash_path.unwrap_or(meta.hash_value);

        Ok(j)
    }

    pub fn rebuild(&self) -> LangResult<Rebuilt> {
        unimplemented!()
    }
}

#[test]
fn test_locr() -> Result<(), LangError> {
    let file = std::fs::read("hash_list.hmla").expect("No file.");
    let hashlist = HashList::load(file.as_slice()).unwrap();

    let locr = LOCR::new(hashlist, Version::H3, None, false)?;
    let filedata = std::fs::read("test.LOCR").expect("No file.");
    let json = locr.convert(
        filedata.as_slice(),
        String::from_utf8(std::fs::read("test.meta.json").expect("No file."))?,
    )?;
    println!("{}", serde_json::to_string(&json)?);

    Ok(())
}
