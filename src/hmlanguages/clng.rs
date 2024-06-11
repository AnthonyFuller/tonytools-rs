use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Map;

use super::{LangError, LangResult, Rebuilt};
use crate::util::{
    rpkg::{self, ResourceMeta},
    vec_of_strings,
};
use crate::Version;
use bitchomp::{ByteReader, ByteWriter, Endianness};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClngJson {
    #[serde(rename = "$schema")]
    schema: String,
    hash: String,
    languages: Map<String, serde_json::Value>,
}

pub struct CLNG {
    lang_map: Vec<String>,
}

impl CLNG {
    pub fn new(version: Version, lang_map: Option<Vec<String>>) -> LangResult<Self> {
        let lang_map = if let Some(map) = lang_map {
            map
        } else {
            match version {
                Version::H2016 | Version::H2 => vec_of_strings![
                    "xx", "en", "fr", "it", "de", "es", "ru", "mx", "br", "pl", "cn", "jp", "tc"
                ],
                Version::H3 => {
                    vec_of_strings!["xx", "en", "fr", "it", "de", "es", "ru", "cn", "tc", "jp"]
                }
                _ => return Err(LangError::UnsupportedVersion),
            }
        };

        Ok(CLNG { lang_map })
    }

    pub fn convert(&self, data: &[u8], meta_json: String) -> LangResult<ClngJson> {
        let mut buf = ByteReader::new(data, Endianness::Little);

        let mut j = ClngJson {
            schema: "https://tonytools.win/schemas/clng.schema.json".into(),
            hash: "".into(),
            languages: Map::new(),
        };

        let bools = buf.read_n::<u8>(buf.len())?;
        let meta: rpkg::ResourceMeta = serde_json::from_str(meta_json.as_str())?;
        j.hash = meta.hash_path.unwrap_or(meta.hash_value);

        for i in 0..bools.len() {
            if i >= self.lang_map.len() {
                return Err(LangError::InvalidLanguageMap);
            }
            let lang = self.lang_map.get(i).unwrap();
            j.languages[lang] = (*bools.get(i).unwrap() == 1u8).into();
        }

        Ok(j)
    }

    pub fn rebuild(&self, json: String) -> LangResult<Rebuilt> {
        let json: ClngJson = serde_json::from_str(&json)?;
        let mut buf = ByteWriter::new(Endianness::Little);

        for v in json.languages.values() {
            if !v.is_boolean() {
                return Err(LangError::InvalidInput);
            }

            buf.append(v.as_bool().unwrap() as u8);
        }

        Ok(Rebuilt {
            file: buf.buf(),
            meta: serde_json::to_string(&ResourceMeta::new(
                json.hash,
                buf.len() as u32,
                "CLNG".into(),
                IndexMap::new(),
            ))?,
        })
    }
}
