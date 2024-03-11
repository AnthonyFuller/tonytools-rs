use super::super::vec_of_strings;
use super::Rebuilt;
use super::{hashlist::HashList, LangError, LangResult};
use crate::util::bytereader::ByteReader;
use crate::util::bytewriter::ByteWriter;
use crate::util::rpkg::{self, is_valid_hash, ResourceMeta};
use crate::util::transmutable::Endianness;
use crate::Version;
use byteorder::LE;
use extended_tea::XTEA;
use fancy_regex::Regex;
use indexmap::{indexmap, IndexMap};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map};

const XTEA: Lazy<XTEA> =
    Lazy::new(|| XTEA::new(&[0x53527737u32, 0x7506499Eu32, 0xBD39AEE3u32, 0xA59E7268u32]));

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DlgeJson {
    #[serde(rename = "$schema")]
    schema: String,
    hash: String,
    #[serde(rename = "DITL")]
    ditl: String,
    #[serde(rename = "CLNG")]
    clng: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    langmap: Option<String>,
    #[serde(rename = "rootContainer")]
    root: DlgeType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WavFile {
    #[serde(rename = "wavName")]
    wav_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<serde_json::Value>,
    soundtag: String,
    #[serde(rename = "defaultWav")]
    default_wav: String,
    #[serde(rename = "defaultFfx")]
    default_ffx: String,
    languages: Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Random {
    cases: Option<Vec<String>>,
    containers: Vec<DlgeType>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Switch {
    #[serde(rename = "switchKey")]
    switch_key: String,
    default: String,
    containers: Vec<DlgeType>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sequence {
    containers: Vec<DlgeType>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DlgeType {
    WavFile(WavFile),
    Random(Random),
    Switch(Switch),
    Sequence(Sequence),
    Null,
}

impl From<WavFile> for DlgeType {
    fn from(v: WavFile) -> Self {
        DlgeType::WavFile(v)
    }
}

impl From<Random> for DlgeType {
    fn from(v: Random) -> Self {
        DlgeType::Random(v)
    }
}

impl From<Switch> for DlgeType {
    fn from(v: Switch) -> Self {
        DlgeType::Switch(v)
    }
}

impl From<Sequence> for DlgeType {
    fn from(v: Sequence) -> Self {
        DlgeType::Sequence(v)
    }
}

pub struct DLGE {
    hashlist: HashList,
    version: Version,
    lang_map: Vec<String>,
    default_locale: String,
    hex_precision: bool,
    custom_langmap: bool,
    // This is used for rebuilding.
    depends: IndexMap<String, String>,
}

#[derive(Clone)]
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
    fn new(r#type: u8, group_hash: u32, default_hash: u32) -> Self {
        Self {
            r#type,
            group_hash,
            default_hash,
            metadata: Vec::new(),
        }
    }

    fn read(buf: &mut ByteReader) -> LangResult<Self> {
        let mut container = Self {
            r#type: buf.read::<u8>()?,
            group_hash: buf.read::<u32>()?,
            default_hash: buf.read::<u32>()?,
            metadata: vec![],
        };

        for _ in 0..buf.read::<u32>()? {
            container.metadata.push(Metadata {
                type_index: buf.read::<u16>()?,
                hashes: buf.read_vec::<u32>()?,
            })
        }

        Ok(container)
    }

    fn write(self, buf: &mut ByteWriter) {
        buf.append(self.r#type);
        buf.append(self.group_hash);
        buf.append(self.default_hash);

        buf.append(self.metadata.len() as u32);
        for metadata in self.metadata {
            buf.append(metadata.type_index);
            buf.write_vec(metadata.hashes);
        }
    }
}

#[derive(Default)]
struct ContainerMap {
    wav: IndexMap<usize, WavFile>,
    random: IndexMap<usize, Random>,
    switch: IndexMap<usize, Switch>,
    sequence: IndexMap<usize, Sequence>,
}

fn get_wav_name(wav_hash: &str, ffx_hash: &str, hash: u32) -> String {
    if is_valid_hash(wav_hash) || is_valid_hash(ffx_hash) {
        return format!("{:08X}", hash);
    }

    let r = Regex::new(r"([^\/]*(?=\.wav))").unwrap();
    let r_ffx = Regex::new(r"([^\/]*(?=\.animset))").unwrap();

    match r.find(wav_hash).unwrap() {
        Some(hash) => hash.as_str().into(),
        None => match r_ffx.find(&ffx_hash).unwrap() {
            Some(hash) => hash.as_str().into(),
            None => format!("{:08X}", hash),
        },
    }
}

fn xtea_encrypt(str: &str) -> Vec<u8> {
    let mut str = str.as_bytes().to_vec();
    if str.len() % 8 != 0 {
        str.extend(vec![0; 8 - (str.len() % 8)]);
    }

    let mut out_data = vec![0; str.len()];

    XTEA.encipher_u8slice::<LE>(&str, &mut out_data);

    out_data
}

impl DLGE {
    pub fn new(
        hashlist: HashList,
        version: Version,
        lang_map: Option<String>,
        default_locale: Option<String>,
        hex_precision: bool,
    ) -> LangResult<Self> {
        let custom_langmap = lang_map.is_some();
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
            custom_langmap,
            depends: IndexMap::new(),
        })
    }

    pub fn convert(&self, data: &[u8], meta_json: String) -> LangResult<DlgeJson> {
        let mut buf = ByteReader::new(data, Endianness::Little);

        let mut j = DlgeJson {
            schema: "https://tonytools.win/schemas/dlge.schema.json".into(),
            hash: "".into(),
            ditl: "".into(),
            clng: "".into(),
            langmap: if self.custom_langmap {
                Some(self.lang_map.join(","))
            } else {
                None
            },
            root: DlgeType::Null,
        };

        let meta: rpkg::ResourceMeta = serde_json::from_str(meta_json.as_str())?;
        j.hash = meta.hash_path.unwrap_or(meta.hash_value);
        j.ditl = meta.hash_reference_data[buf.read::<u32>()? as usize]
            .hash
            .clone();
        j.clng = meta.hash_reference_data[buf.read::<u32>()? as usize]
            .hash
            .clone();

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
        let mut globals: IndexMap<u32, usize> = IndexMap::new();

        while buf.cursor.len() != 2 {
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
                                wav.default_wav =
                                    meta.hash_reference_data[wav_index as usize].hash.clone();
                                wav.default_ffx =
                                    meta.hash_reference_data[ffx_index as usize].hash.clone();

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
                                .to_string()
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

                    containers.wav.insert(indices[1], wav);
                    indices[1] += 1;
                }
                0x02 => {
                    let container = Container::read(&mut buf)?;
                    let mut random = Random {
                        cases: None,
                        containers: vec![],
                    };

                    for metadata in container.metadata {
                        let r#type = metadata.type_index >> 12;
                        let index = metadata.type_index & 0xFFF;

                        if r#type != 0x01 {
                            return Err(LangError::InvalidReference(r#type as u8));
                        }

                        containers.wav[index as usize].weight = match self.hex_precision {
                            true => Some(format!("{:06X}", metadata.hashes[0]).into()),
                            false => Some(((metadata.hashes[0] as f64) / (0xFFFFFF as f64)).into()),
                        };

                        random
                            .containers
                            .push(containers.wav[index as usize].clone().into());
                        containers.wav.shift_remove(&(index as usize));
                    }

                    containers.random.insert(indices[2], random);
                    global_index += 1;
                    globals[&global_index] = indices[2];
                    indices[2] += 1;
                }
                0x03 => {
                    let container = Container::read(&mut buf)?;
                    let mut switch = Switch {
                        switch_key: self
                            .hashlist
                            .tags
                            .get_by_left(&container.group_hash)
                            .unwrap_or(&format!("{:08X}", container.group_hash))
                            .clone(),
                        default: self
                            .hashlist
                            .switches
                            .get_by_left(&container.default_hash)
                            .unwrap_or(&format!("{:08X}", container.default_hash))
                            .clone(),
                        containers: vec![],
                    };

                    for metadata in container.metadata {
                        // Switch containers will ONLY EVER CONTAIN references to random containers. And there will only ever be 1 per DLGE.
                        // But, they may contain more than one entry (or no entries) in the "SwitchHashes" array.
                        // This has been verified across all games. This, again, makes sense when considering the purposes of each container.
                        // But, we allow WavFile references in HMLT as they make sense, but currently it's unknown if the game allows for this.

                        let r#type = metadata.type_index >> 12;
                        let index = metadata.type_index & 0xFFF;

                        if r#type != 0x01 && r#type != 0x02 {
                            return Err(LangError::InvalidReference(r#type as u8));
                        }

                        let mut cases: Vec<String> = vec![];
                        for hash in metadata.hashes {
                            cases.push(
                                self.hashlist
                                    .switches
                                    .get_by_left(&hash)
                                    .unwrap_or(&format!("{:08X}", hash))
                                    .clone(),
                            )
                        }

                        match r#type {
                            0x01 => {
                                containers.wav[index as usize].cases = cases.into();
                                switch.containers.push(DlgeType::WavFile(
                                    containers.wav[index as usize].clone(),
                                ));
                                containers.wav.shift_remove(&(index as usize));
                            }
                            0x02 => {
                                containers.random[index as usize].cases = cases.into();
                                switch
                                    .containers
                                    .push(containers.random[index as usize].clone().into());
                                containers.random.shift_remove(&(index as usize));
                            }
                            _ => {}
                        }
                    }

                    containers.switch.insert(indices[3], switch);
                    global_index += 1;
                    globals[&global_index] = indices[3];
                    indices[3] += 1;
                }
                0x04 => {
                    // Sequence containers can contain any of the containers apart from sequence containers of course.
                    // Unsure if this is a hard limitation, or if they've just not used any.
                    // Further testing required. (Although if it is a limitation, this is logical).
                    let container = Container::read(&mut buf)?;
                    let mut sequence = Sequence { containers: vec![] };

                    for metadata in container.metadata {
                        let r#type = metadata.type_index >> 12;
                        if r#type == 0x04 {
                            return Err(LangError::InvalidReference(r#type as u8));
                        }

                        let index = match r#type {
                            0x02 | 0x03 => globals[&((metadata.type_index & 0xFFF) as u32)],
                            _ => (metadata.type_index & 0xFFF) as usize,
                        };

                        match r#type {
                            0x01 => {
                                sequence
                                    .containers
                                    .push(containers.wav[index].clone().into());
                                containers.wav.shift_remove(&index);
                            }
                            0x02 => {
                                sequence
                                    .containers
                                    .push(containers.random[index].clone().into());
                                containers.random.shift_remove(&index);
                            }
                            0x03 => {
                                sequence
                                    .containers
                                    .push(containers.switch[index].clone().into());
                                containers.switch.shift_remove(&index);
                            }
                            _ => {}
                        }
                    }

                    containers.sequence.insert(indices[4], sequence);
                    indices[4] += 1;
                }
                n => return Err(LangError::InvalidContainer(n)),
            }
        }

        let root = buf.read::<u16>()?;
        let root_type = root >> 12;
        let root_index = (root & 0xFFF) as usize;

        j.root = match root_type {
            0x01 => containers.wav[root_index].clone().into(),
            0x02 => containers.random[root_index].clone().into(),
            0x03 => containers.switch[root_index].clone().into(),
            0x04 => containers.sequence[root_index].clone().into(),
            n => return Err(LangError::InvalidContainer(n as u8)),
        };

        Ok(j)
    }

    fn add_depend(&mut self, path: String, flag: String) -> u32 {
        if self.depends.contains_key(&path) {
            return self.depends.get_index_of(&path).unwrap() as u32;
        } else {
            self.depends.insert(path, flag);
            return (self.depends.len() - 1) as u32;
        }
    }

    fn process_container(
        &mut self,
        buf: &mut ByteWriter,
        container: &mut DlgeType,
        indices: &mut IndexMap<i32, i32>,
        is_root: bool,
    ) -> LangResult<()> {
        match container {
            DlgeType::WavFile(wav) => {
                buf.append::<u8>(0x01);
                buf.append::<u32>(*self.hashlist.tags.get_by_right(&wav.soundtag).unwrap());
                buf.append::<u32>(
                    u32::from_str_radix(&wav.wav_name, 16)
                        .unwrap_or(crc32fast::hash(wav.wav_name.as_bytes())),
                );

                if self.version != Version::H2016 {
                    buf.append::<u32>(0x00);
                }

                for (index, language) in self.lang_map.clone().iter().enumerate() {
                    if self.version == Version::H2016 {
                        buf.append::<u32>(0x00);
                    }

                    if *language == self.default_locale {
                        buf.append(
                            self.add_depend(
                                wav.default_wav.clone(),
                                format!("{:02X}", 0x80 + index),
                            ),
                        );
                        buf.append(
                            self.add_depend(
                                wav.default_ffx.clone(),
                                format!("{:02X}", 0x80 + index),
                            ),
                        );

                        if wav.languages.contains_key(language) {
                            match wav.languages[language].as_str() {
                                Some(str) => {
                                    if str.len() == 0 {
                                        buf.append::<u32>(0);
                                    }

                                    buf.write_vec(xtea_encrypt(str));
                                }
                                None => {
                                    buf.append::<u32>(0);
                                }
                            }
                        }
                    } else {
                        if !wav.languages.contains_key(language) {
                            buf.append::<u64>(u64::MAX);
                            buf.append::<u32>(0);

                            continue;
                        }

                        match wav.languages[language].as_object() {
                            Some(obj) => {
                                buf.append(self.add_depend(
                                    obj["wav"].to_string(),
                                    format!("{:02X}", 0x80 + index),
                                ));
                                buf.append(self.add_depend(
                                    obj["ffx"].to_string(),
                                    format!("{:02X}", 0x80 + index),
                                ));

                                if obj.contains_key("subtitle") {
                                    let subtitle = obj["subtitle"].as_str().unwrap();
                                    buf.write_vec(xtea_encrypt(subtitle));
                                } else {
                                    buf.append::<u32>(0);
                                }

                                continue;
                            }
                            None => {
                                buf.append::<u64>(u64::MAX);

                                if wav.languages[language].is_string() {
                                    let subtitle = wav.languages[language].as_str().unwrap();
                                    buf.write_vec(xtea_encrypt(subtitle));
                                } else {
                                    buf.append::<u32>(0);
                                }

                                continue;
                            }
                        }
                    }
                }

                indices[1] += 1;
            }
            DlgeType::Random(random) => {
                let mut container = Container::new(0x02, 0, 0);

                for child in random.containers.clone() {
                    match child {
                        DlgeType::WavFile(wav) => {
                            if wav.weight.is_none() {
                                return Err(LangError::InvalidReference(0x01));
                            }

                            let weight_value = wav.weight.clone().unwrap();

                            self.process_container(buf, &mut wav.clone().into(), indices, false)?;

                            let weight: u32 = match weight_value.as_str() {
                                Some(str) => u32::from_str_radix(str, 16)?,
                                None => {
                                    // Must be double
                                    let value = weight_value.as_f64().unwrap();
                                    (value * (0xFFFFFF as f64)).round() as u32
                                }
                            };

                            container.metadata.push(Metadata {
                                type_index: ((0x02 << 12) | (indices[2] & 0xFFF)) as u16,
                                hashes: vec![weight],
                            });
                        }
                        _ => {
                            return Err(LangError::InvalidReference(0x15));
                        }
                    }
                }

                container.write(buf);
                indices[0] += 1;
                indices[2] += 1;
            }
            DlgeType::Switch(switch) => {
                if indices[3] != -1 {
                    return Err(LangError::InvalidContainer(0x03));
                }

                let mut container = Container::new(
                    3,
                    *self
                        .hashlist
                        .switches
                        .get_by_right(&switch.switch_key)
                        .unwrap_or(
                            &u32::from_str_radix(&switch.switch_key, 16)
                                .unwrap_or(crc32fast::hash(switch.switch_key.as_bytes())),
                        ),
                    *self
                        .hashlist
                        .switches
                        .get_by_right(&switch.default)
                        .unwrap_or(
                            &u32::from_str_radix(&switch.default, 16)
                                .unwrap_or(crc32fast::hash(switch.default.as_bytes())),
                        ),
                );

                for child in switch.containers.clone() {
                    let source_cases: Vec<String>;
                    let mut cases: Vec<u32> = Vec::new();

                    match child.clone() {
                        DlgeType::WavFile(container) => {
                            if container.cases.is_none() {
                                return Err(LangError::InvalidReference(0x15));
                            }
                            source_cases = container.cases.unwrap();
                        }
                        DlgeType::Random(container) => {
                            if container.cases.is_none() {
                                return Err(LangError::InvalidReference(0x15));
                            }
                            source_cases = container.cases.unwrap();
                        }
                        _ => {
                            return Err(LangError::InvalidReference(0x15));
                        }
                    }

                    self.process_container(buf, &mut child.clone(), indices, false)?;

                    for case in source_cases {
                        cases.push(
                            *self.hashlist.switches.get_by_right(&case).unwrap_or(
                                &u32::from_str_radix(&case, 16)
                                    .unwrap_or(crc32fast::hash(case.as_bytes())),
                            ),
                        );
                    }

                    container.metadata.push(Metadata {
                        type_index: ((0x03 << 12) | (indices[3] & 0xFFF)) as u16,
                        hashes: cases,
                    });

                    indices[3] += 1;
                }

                container.write(buf);
                indices[0] += 1;
                indices[3] += 1;
            }
            DlgeType::Sequence(sequence) => {
                if indices[4] != -1 {
                    return Err(LangError::InvalidContainer(0x04));
                }

                let container = Container::new(4, 0, 0);

                for child in sequence.containers.clone() {
                    self.process_container(buf, &mut child.clone(), indices, false)?;

                    indices[4] += 1;
                }

                container.write(buf);
                indices[0] += 1;
                indices[4] += 1;
            }
            _ => {}
        }

        if is_root {
            let container_type = match container {
                DlgeType::WavFile(_) => 1,
                DlgeType::Random(_) => 2,
                DlgeType::Switch(_) => 3,
                DlgeType::Sequence(_) => 4,
                _ => 0x15,
            };
            buf.append::<u16>(
                ((container_type << 12)
                    | (indices[if container_type == 0x01 { 0x01 } else { 0x00 }] & 0xFFF))
                    as u16,
            );
        }

        Ok(())
    }

    pub fn rebuild(&mut self, json: String) -> LangResult<Rebuilt> {
        self.depends.clear();

        let mut json: DlgeJson = serde_json::from_str(&json)?;

        // The langmap property overrides the struct's language map.
        // This property ensures easy compat with tools like SMF.
        // We restore this back later.
        let mut old_langmap: Option<Vec<String>> = None;
        if json.langmap.is_some() {
            old_langmap = Some(self.lang_map.clone());
            self.lang_map = json
                .langmap
                .unwrap()
                .split(",")
                .map(|s| s.to_string())
                .collect();
        };

        let mut buf = ByteWriter::new(Endianness::Little);

        buf.append::<u32>(0x00);
        self.depends.insert(json.ditl, String::from("1F"));
        buf.append::<u32>(0x01);
        self.depends.insert(json.clng, String::from("1F"));

        // 0 is the "global" index
        let mut indices = indexmap! {
            0 => -1,
            1 => -1,
            2 => -1,
            3 => -1,
            4 => -1
        };

        self.process_container(&mut buf, &mut json.root, &mut indices, true)?;

        if old_langmap.is_some() {
            self.lang_map = old_langmap.unwrap();
        }

        Ok(Rebuilt {
            file: buf.buf(),
            meta: serde_json::to_string(&ResourceMeta::new(
                json.hash,
                buf.len() as u32,
                "DLGE".into(),
                self.depends.clone(),
            ))?,
        })
    }
}
