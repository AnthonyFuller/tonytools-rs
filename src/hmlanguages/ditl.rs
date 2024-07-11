use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Map;

use super::hashlist::HashList;
use super::{LangResult, Rebuilt};
use crate::util::rpkg::{self, ResourceMeta};
use bitchomp::{ByteReader, ByteWriter, Endianness, ChompFlatten};

#[derive(Serialize, Deserialize, Debug)]
pub struct DitlJson {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub hash: String,
    pub soundtags: Map<String, serde_json::Value>,
}

pub struct DITL {
    hashlist: HashList,
    // This is used for rebuilding.
    depends: IndexMap<String, String>,
}

impl DITL {
    pub fn new(hashlist: HashList) -> LangResult<Self> {
        Ok(DITL {
            hashlist,
            depends: IndexMap::new(),
        })
    }

    pub fn convert(&self, data: &[u8], meta_json: String) -> LangResult<DitlJson> {
        let mut buf = ByteReader::new(data, Endianness::Little);

        let mut j = DitlJson {
            schema: "https://tonytools.win/schemas/ditl.schema.json".into(),
            hash: "".into(),
            soundtags: Map::new(),
        };

        let count = buf.read::<u32>()?.inner();
        let hashes = buf.read_n::<u32>((count * 2) as usize)?.flatten(); // Hashes and depend index
        let meta: rpkg::ResourceMeta = serde_json::from_str(meta_json.as_str())?;
        j.hash = meta.hash_path.unwrap_or(meta.hash_value);

        for i in (0..hashes.len()).step_by(2) {
            let index = *hashes.get(i).unwrap();
            let hash = *hashes.get(i + 1).unwrap();
            let depend = meta
                .hash_reference_data
                .get(index as usize)
                .unwrap()
                .clone();
            let hex: String = format!("{:08X}", hash);
            let hash = self.hashlist.tags.get_by_left(&hash).unwrap_or(&hex);
            j.soundtags.insert(hash.clone(), depend.hash.into());
        }

        Ok(j)
    }

    fn add_depend(&mut self, path: String, flag: String) -> u32 {
        if self.depends.contains_key(&path) {
            self.depends.get_index_of(&path).unwrap() as u32
        } else {
            self.depends.insert(path, flag);
            (self.depends.len() - 1) as u32
        }
    }

    pub fn rebuild(&mut self, json: String) -> LangResult<Rebuilt> {
        self.depends.clear();
        let json: DitlJson = serde_json::from_str(&json)?;

        let mut buf = ByteWriter::new(Endianness::Little);

        buf.append(json.soundtags.len() as u32);

        for (tag, hash) in json.soundtags {
            let hash = hash.as_str().unwrap();

            buf.append(self.add_depend(hash.to_string(), "1F".into()));
            buf.append(*self.hashlist.tags.get_by_right(&tag).unwrap_or(
                &u32::from_str_radix(&tag, 16).unwrap_or(crc32fast::hash(tag.as_bytes())),
            ));
        }

        Ok(Rebuilt {
            file: buf.buf(),
            meta: serde_json::to_string(&ResourceMeta::new(
                json.hash,
                buf.len() as u32,
                "DITL".into(),
                self.depends.clone(),
            ))?,
        })
    }
}
