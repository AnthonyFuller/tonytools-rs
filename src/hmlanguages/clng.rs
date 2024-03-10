use serde::{Deserialize, Serialize};
use serde_json::Map;

use super::{LangError, LangResult, Rebuilt};
use crate::util::bytereader::ByteReader;
use crate::util::rpkg;
use crate::util::transmutable::Endianness;
use crate::{vec_of_strings, Version};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClngJson {
    #[serde(rename = "$schema")]
    schema: String,
    hash: String,
    languages: serde_json::Value,
}

pub struct CLNG {
    lang_map: Vec<String>,
}

impl CLNG {
    pub fn new(version: Version, lang_map: Option<String>) -> LangResult<Self> {
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

        Ok(CLNG { lang_map })
    }

    pub fn convert(&self, data: &[u8], meta_json: String) -> LangResult<ClngJson> {
        let mut buf = ByteReader::new(data, Endianness::Little);

        let mut j = ClngJson {
            schema: "https://tonytools.win/schemas/clng.schema.json".into(),
            hash: "".into(),
            languages: Map::new().into(),
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

    pub fn rebuild(&self) -> LangResult<Rebuilt> {
        unimplemented!()
    }
}

#[test]
fn test_clng() -> LangResult<()> {
    let clng = CLNG::new(Version::H3, None)?;
    let filedata = std::fs::read("test.CLNG").expect("No file.");
    let json = clng.convert(
        filedata.as_slice(),
        String::from_utf8(std::fs::read("test.clng.json").expect("No file."))?,
    )?;
    println!("{}", serde_json::to_string(&json)?);

    Ok(())
}
