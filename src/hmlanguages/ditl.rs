use serde::{Deserialize, Serialize};
use serde_json::Map;

use super::hashlist::HashList;
use super::{LangError, LangResult, Rebuilt};
use crate::util::bytereader::{ByteReader, Endianness};
use crate::util::rpkg;

#[derive(Serialize, Deserialize, Debug)]
pub struct DitlJson {
    #[serde(rename = "$schema")]
    schema: String,
    hash: String,
    soundtags: serde_json::Value,
}

pub struct DITL {
    hashlist: HashList,
}

impl DITL {
    pub fn new(hashlist: HashList) -> LangResult<Self> {
        Ok(DITL { hashlist })
    }

    pub fn convert(&self, data: &[u8], meta_json: String) -> LangResult<DitlJson> {
        let mut buf = ByteReader::new(data, Endianness::Little);

        let mut j = DitlJson {
            schema: "https://tonytools.win/schemas/ditl.schema.json".into(),
            hash: "".into(),
            soundtags: Map::new().into(),
        };

        let count = buf.read::<u32>()?;
        let hashes = buf.read_n::<u32>((count * 2) as usize)?; // Hashes and depend index
        let meta: rpkg::ResourceMeta = serde_json::from_str(meta_json.as_str())?;
        j.hash = meta.hash_path.unwrap_or(meta.hash_value);

        for i in (0..hashes.len()).step_by(2) {
            let index = *hashes.get(i).unwrap();
            let hash = *hashes.get(i + 1).unwrap();
            let depend = meta.hash_reference_data.get(index as usize).unwrap().clone();
            let hex: String = format!("{:08X}", hash);
            let hash = self.hashlist.tags.get_by_left(&hash).unwrap_or(&hex);
            j.soundtags[hash] = depend.hash.into();
        }

        Ok(j)
    }

    pub fn rebuild(&self) -> LangResult<Rebuilt> {
        unimplemented!()
    }
}

#[test]
fn test_ditl() -> Result<(), LangError> {
    let file = std::fs::read("hash_list.hmla").expect("No file.");
    let hashlist = HashList::load(file.as_slice()).unwrap();

    let ditl = DITL::new(hashlist)?;
    let filedata = std::fs::read("test.DITL").expect("No file.");
    let json = ditl.convert(
        filedata.as_slice(),
        String::from_utf8(std::fs::read("test.ditl.json").expect("No file."))?,
    )?;
    println!("{}", serde_json::to_string(&json)?);

    Ok(())
}
