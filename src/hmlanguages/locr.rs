use super::super::vec_of_strings;
use super::Rebuilt;
use super::{hashlist::HashList, LangError, LangResult};
use crate::util::bytereader::ByteReader;
use crate::util::bytewriter::ByteWriter;
use crate::util::cipher::{symmetric_decrypt, symmetric_encrypt, xtea_decrypt, xtea_encrypt};
use crate::util::rpkg::{self, ResourceMeta};
use crate::util::transmutable::Endianness;
use crate::Version;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Map;

#[derive(Serialize, Deserialize, Debug)]
pub struct LocrJson {
    #[serde(rename = "$schema")]
    schema: String,
    hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symmetric: Option<bool>,
    languages: Map<String, serde_json::Value>,
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
            lang_map
                .unwrap()
                .split(",")
                .map(|s| s.to_string())
                .collect()
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

        let mut j = LocrJson {
            schema: "https://tonytools.win/schemas/locr.schema.json".into(),
            hash: "".into(),
            symmetric: None,
            languages: Map::new().into(),
        };

        if self.symmetric && self.version == Version::H2016 {
            j.symmetric = Some(true);
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
            j.languages.insert(language.clone(), Map::new().into());

            if offsets[i] == u32::MAX {
                continue;
            }
            buf.seek(offsets[i] as usize)?;

            for _ in 0..buf.read::<u32>()? {
                let hash_num = buf.read::<u32>()?;
                let hex: String = format!("{:08X}", hash_num);
                let hash = self.hashlist.lines.get_by_left(&hash_num).unwrap_or(&hex);
                let str_data = buf.read_vec::<u8>()?;
                buf.seek(buf.cursor() + 1)?; // Skip null terminator

                j.languages[language][hash] = match self.symmetric {
                    true => symmetric_decrypt(str_data)?.into(),
                    false => xtea_decrypt(str_data)?.into(),
                }
            }
        }

        let meta: rpkg::ResourceMeta = serde_json::from_str(meta_json.as_str())?;
        j.hash = meta.hash_path.unwrap_or(meta.hash_value);

        Ok(j)
    }

    pub fn rebuild(&self, json: String) -> LangResult<Rebuilt> {
        let json: LocrJson = serde_json::from_str(&json)?;
        let mut symmetric = self.symmetric;

        if json.symmetric.is_some_and(|b| b) && self.version == Version::H2016 {
            symmetric = true;
        }

        let mut buf = ByteWriter::new(Endianness::Little);

        if self.version != Version::H2016 {
            buf.append::<u8>(0);
        }

        let mut offset = buf.len();

        buf.write_vec(vec![0; json.languages.len()]);

        for strings in json.languages.values() {
            if !strings.is_object() {
                return Err(LangError::InvalidInput);
            }
            let strings = strings.as_object().unwrap();

            if strings.len() == 0 {
                buf.write(u32::MAX, offset)?;
                offset += 4;
                continue;
            }

            buf.write(buf.len() as u32, offset)?;
            offset += 4;

            buf.append(strings.len() as u32);
            for (hash, str) in strings {
                if !str.is_string() {
                    return Err(LangError::InvalidInput);
                }
                let str = str.as_str().unwrap();

                buf.append(*self.hashlist.lines.get_by_right(hash).unwrap_or(
                    &u32::from_str_radix(hash, 16).unwrap_or(crc32fast::hash(hash.as_bytes())),
                ));
                buf.write_sized_vec(match symmetric {
                    true => symmetric_encrypt(str.as_bytes().to_vec()),
                    false => xtea_encrypt(str),
                });
                buf.append::<u8>(0);
            }
        }

        Ok(Rebuilt {
            file: buf.buf(),
            meta: serde_json::to_string(&ResourceMeta::new(
                json.hash,
                buf.len() as u32,
                "LOCR".into(),
                IndexMap::new(),
            ))?,
        })
    }
}
