use std::collections::HashMap;
use std::ptr::null;

use super::super::vec_of_strings;
use super::Rebuilt;
use super::{hashlist::HashList, LangError, LangResult};
use crate::util::bytereader::{ByteReader, Endianness};
use crate::util::rpkg::{self, is_valid_hash};
use crate::Version;
use byteorder::LE;
use extended_tea::XTEA;
use indexmap::{indexmap, IndexMap};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

const XTEA: Lazy<XTEA> =
    Lazy::new(|| XTEA::new(&[0x53527737u32, 0x7506499Eu32, 0xBD39AEE3u32, 0xA59E7268u32]));

#[derive(Serialize, Deserialize, Debug)]
pub struct DlgeJson {
    #[serde(rename = "$schema")]
    schema: String,
    hash: String,
    #[serde(rename = "DITL")]
    ditl: String,
    #[serde(rename = "CLNG")]
    clng: String,
    #[serde(rename = "rootContainer")]
    root: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WavFile {
    #[serde(rename = "wavName")]
    wav_name: String,
    cases: Option<Vec<String>>,
    weight: Option<serde_json::Value>,
    soundtag: String,
    #[serde(rename = "defaultWav")]
    default_wav: String,
    #[serde(rename = "defaultFfx")]
    default_ffx: String,
    languages: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Random {
    cases: Option<Vec<String>>,
    containers: Vec<DlgeType>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Switch {
    #[serde(rename = "switchKey")]
    switch_key: String,
    default: String,
    containers: Vec<DlgeType>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sequence {
    containers: Vec<DlgeType>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum DlgeType {
    WavFile(WavFile),
    Random(Random),
    Switch(Switch),
    Sequence(Sequence),
}

pub struct DLGE {
    hashlist: HashList,
    version: Version,
    lang_map: Vec<String>,
    default_locale: String,
    hex_precision: bool,
}

struct Metadata {
    type_index: u16, // >> 12 for type -- & 0xFFF for index
    // This is actually a u32 count, then X amount of u32s but
    // our bytereader, when reading a vector, reads a u32 of size first.
    hashes: Vec<u32>,
}

struct Container {
    r#type: u8,
    group_hash: u32,
    default_hash: u32,
    metadata: Vec<Metadata>,
}

impl Container {
    fn read(mut buf: ByteReader) -> LangResult<Self> {
        let mut container = Self {
            r#type: buf.read::<u8>()?,
            group_hash: buf.read::<u32>()?,
            default_hash: buf.read::<u32>()?,
            metadata: vec![]
        };

        for _ in 0..buf.read::<u32>()? {
            container.metadata.push(Metadata {
                type_index: buf.read::<u16>()?,
                hashes: buf.read_vec::<u32>()?
            })
        }

        Ok(container)
    }
}

#[derive(Default)]
struct ContainerMap {
    wav: IndexMap<u32, WavFile>,
    random: IndexMap<u32, Random>,
    switch: IndexMap<u32, Switch>,
    sequence: IndexMap<u32, Sequence>,
}

fn get_wav_name(wav_hash: &str, ffx_hash: &str, hash: u32) -> String {
    if is_valid_hash(wav_hash) || is_valid_hash(ffx_hash) {
        return format!("{:08X}", hash);
    }

    let wav = Regex::new(r"([^\/]*(?=\.wav))").unwrap();
    let ffx = Regex::new(r"([^\/]*(?=\.animset))").unwrap();

    match wav.find(wav_hash) {
        Some(hash) => hash.as_str().into(),
        None => match ffx.find(&ffx_hash) {
            Some(hash) => hash.as_str().into(),
            None => format!("{:08X}", hash),
        },
    }
}

impl DLGE {
    pub fn new(
        hashlist: HashList,
        version: Version,
        lang_map: Option<String>,
        default_locale: Option<String>,
        hex_precision: bool,
    ) -> LangResult<Self> {
        let lang_map = if lang_map.is_none() {
            match version {
                Version::H2016 => vec_of_strings![
                    "xx", "en", "fr", "it", "de", "es", "ru", "mx", "br", "pl", "cn", "jp"
                ],
                Version::H2 => vec_of_strings![
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

        let default_locale = default_locale.unwrap_or(String::from("en"));

        Ok(DLGE {
            hashlist,
            version,
            lang_map,
            default_locale,
            hex_precision,
        })
    }

    pub fn convert(&self, data: &[u8], meta_json: String) -> LangResult<DlgeJson> {
        let mut buf = ByteReader::new(data, Endianness::Little);

        let mut j: DlgeJson = serde_json::from_str(
            r#"{
            "$schema": "https://tonytools.win/schemas/dlge.schema.json",
            "hash": "",
            "DITL": "",
            "CLNG": "",
            "rootContainer": null
        }"#,
        )
        .expect("Something has gone horribly wrong.");

        let meta: rpkg::ResourceMeta = serde_json::from_str(meta_json.as_str())?;
        j.hash = meta.hash_path.unwrap_or(meta.hash_value);
        j.ditl = meta
            .hash_reference_data
            .get(buf.read::<u32>()? as usize)
            .unwrap()
            .clone()
            .hash;
        j.clng = meta
            .hash_reference_data
            .get(buf.read::<u32>()? as usize)
            .unwrap()
            .clone()
            .hash;

        // We setup these maps to store the various types of containers
        // and the latest index for final construction later.
        let mut containers = ContainerMap::default();
        let mut indices = indexmap! {
            1 => 0,
            2 => 0,
            3 => 0,
            4 => 0
        };

        // Weirdly, sequences reference by some "global id" for certain types so we store this here.
        let mut global_index: u32 = u32::MAX;
        let mut globals: HashMap<u32, u32> = HashMap::new();

        while buf.cursor() != (buf.len() - 2) {
            match buf.peek::<u8>()? {
                0x01 => {
                    buf.seek(buf.cursor() + 1)?;
                    let tag_hash = buf.read::<u32>()?;
                    let wav_hash = buf.read::<u32>()?;

                    if self.version != Version::H2016 {
                        buf.read::<u32>()?;
                    }

                    let mut wav = WavFile {
                        wav_name: format!("{:08X}", wav_hash),
                        cases: None,
                        weight: None,
                        soundtag: self
                            .hashlist
                            .tags
                            .get_by_left(&tag_hash)
                            .unwrap_or(&format!("{:08X}", tag_hash))
                            .clone(),
                        default_wav: String::from(""),
                        default_ffx: String::from(""),
                        languages: Map::new().into(),
                    };

                    for language in self.lang_map.as_slice() {
                        if self.version == Version::H2016 {
                            buf.read::<u32>()?;
                        }

                        let wav_index = buf.read::<u32>()?;
                        let ffx_index = buf.read::<u32>()?;

                        let mut subtitle: serde_json::Value = serde_json::Value::Null;

                        if wav_index != u32::MAX && ffx_index != u32::MAX {
                            if *language == self.default_locale {
                                wav.default_wav = meta
                                    .hash_reference_data
                                    .get(wav_index as usize)
                                    .unwrap()
                                    .clone()
                                    .hash;

                                wav.default_ffx = meta
                                    .hash_reference_data
                                    .get(ffx_index as usize)
                                    .unwrap()
                                    .clone()
                                    .hash;

                                wav.wav_name =
                                    get_wav_name(&wav.default_wav, &wav.default_ffx, wav_hash);
                            } else {
                                subtitle = json!({
                                    "wav": meta
                                        .hash_reference_data
                                        .get(wav_index as usize)
                                        .unwrap()
                                        .clone()
                                        .hash,
                                    "ffx": meta
                                        .hash_reference_data
                                        .get(ffx_index as usize)
                                        .unwrap()
                                        .clone()
                                        .hash
                                })
                            }
                        }

                        if buf.peek::<u32>()? != 0 {
                            let str_data = buf.read_vec::<u8>()?;
                            let mut out_data = str_data.clone();

                            XTEA.decipher_u8slice::<LE>(&str_data, &mut out_data);
                            let data: serde_json::Value = String::from_utf8(out_data)?
                                .trim_matches(char::from(0))
                                .into();

                            if subtitle.is_null() {
                                subtitle = data;
                            } else {
                                subtitle["subtitle"] = data;
                            }
                        } else {
                            buf.seek(buf.cursor() + 4)?;
                        }

                        if !subtitle.is_null() {
                            wav.languages[language] = subtitle;
                        }
                    }

                    containers.wav.insert(indices[0], wav);
                    indices[0] += 1;
                }
                0x02 => {
                    let container = Container::read(buf.clone())?;
                    let mut random = Random {
                        cases: None,
                        containers: vec![]
                    };

                    for metadata in container.metadata {
                        let r#type = metadata.type_index >> 12;
                        let index = metadata.type_index & 0xFFF;

                        if r#type != 0x01 {
                            return Err(LangError::InvalidContainer(r#type as u8));
                        }
                    }
                }
                0x03 => {}
                0x04 => {}
                n => return Err(LangError::InvalidContainer(n)),
            }
        }

        Ok(j)
    }

    pub fn rebuild(&self) -> LangResult<Rebuilt> {
        unimplemented!()
    }
}

#[test]
fn test_dlge() -> Result<(), LangError> {
    let file = std::fs::read("hash_list.hmla").expect("No file.");
    let hashlist = HashList::load(file.as_slice()).unwrap();

    let dlge = DLGE::new(hashlist, Version::H3, None, None, false)?;
    let filedata = std::fs::read("test.DLGE").expect("No file.");
    let json = dlge.convert(
        filedata.as_slice(),
        String::from_utf8(std::fs::read("test.dlge.json").expect("No file."))?,
    )?;
    println!("{}", serde_json::to_string(&json)?);

    Ok(())
}
