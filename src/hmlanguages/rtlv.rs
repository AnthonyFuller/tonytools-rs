use bitchomp::{ByteReader, ByteWriter, Endianness};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Map;

use crate::{
    util::{
        cipher::{xtea_decrypt, xtea_encrypt},
        rpkg::{compute_hash, is_valid_hash, ResourceMeta},
    },
    vec_of_strings, Version,
};

use super::{LangError, LangResult, Rebuilt};

#[derive(Serialize, Deserialize, Debug)]
pub struct RtlvJson {
    #[serde(rename = "$schema")]
    schema: String,
    hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    langmap: Option<String>,
    videos: Map<String, serde_json::Value>,
    subtitles: Map<String, serde_json::Value>,
}

// This is a knockoff of the ZHMSerializer from ZHMTools.
// Means I don't have to bind.
#[derive(Debug)]
struct GameRtlv {
    video_languages: Vec<String>,
    video_rids: Vec<u64>,
    subtitle_languages: Vec<String>,
    subtitles: Vec<String>,
    relocations: Vec<u32>,
}

impl GameRtlv {
    pub fn read(buf: &mut ByteReader) -> LangResult<Self> {
        let video_languages = Self::read_string_vec(buf)?;
        let video_rids = Self::read_rid_vec(buf)?;
        let subtitle_languages = Self::read_string_vec(buf)?;
        let subtitles = Self::read_string_vec(buf)?;
        Ok(GameRtlv {
            video_languages,
            video_rids,
            subtitle_languages,
            subtitles,
            relocations: Vec::new(),
        })
    }

    pub fn serialize(&mut self) -> LangResult<Vec<u8>> {
        let mut buf = ByteWriter::new(Endianness::Little);

        // Write bytes for the pointers we change later.
        buf.write_vec(vec![0_u64; 12]);

        // Write video languages
        let offset = buf.len();
        self.write_vec_ptrs(
            &mut buf,
            0x00,
            offset as u64,
            (self.video_languages.len() * 16) as u64,
        )?;
        buf.write_vec(self.write_string_vec(self.video_languages.clone(), offset)?);

        // Write video rids
        let offset = buf.len();
        self.write_vec_ptrs(
            &mut buf,
            0x18,
            offset as u64,
            (self.video_rids.len() * 8) as u64,
        )?;
        for id in self.video_rids.iter() {
            buf.append((*id >> 32) as u32);
            buf.append((*id & u32::MAX as u64) as u32);
        }

        // Write subtitle languages
        let offset = buf.len();
        self.write_vec_ptrs(
            &mut buf,
            0x30,
            offset as u64,
            (self.subtitle_languages.len() * 16) as u64,
        )?;
        buf.write_vec(self.write_string_vec(self.subtitle_languages.clone(), offset)?);

        // Write subtitles
        let offset = buf.len();
        self.write_vec_ptrs(
            &mut buf,
            0x48,
            offset as u64,
            (self.subtitles.len() * 16) as u64,
        )?;
        buf.write_vec(self.write_string_vec(self.subtitles.clone(), offset)?);

        // Since we are done writing data that is included in the file size.
        // we make the header now.

        let mut bin = ByteWriter::new(Endianness::Big);
        // Write header
        let header: Vec<u8> = vec![0x42, 0x49, 0x4E, 0x31, 0x00, 0x08, 0x01, 0x00];
        bin.write_vec(header);
        // Write size
        bin.append(buf.len() as u32);
        bin.append(0_u32);

        // Write relocations
        buf.append(0x12EBA5ED_u32);
        self.relocations.sort();
        buf.append(((self.relocations.len() * 4) + 4) as u32);
        buf.write_sized_vec(self.relocations.clone());

        let mut file = bin.buf();
        file.append(&mut buf.buf());
        Ok(file)
    }

    fn write_vec_ptrs(
        &mut self,
        buf: &mut ByteWriter,
        pos: usize,
        start: u64,
        size: u64,
    ) -> LangResult<()> {
        buf.write(start, pos)?;
        buf.write(start + size, pos + 8)?;
        buf.write(start + size, pos + 16)?;
        let pos = pos as u32;
        self.relocations.append(&mut vec![pos, pos + 8, pos + 16]);

        Ok(())
    }

    fn write_string_vec(&mut self, data: Vec<String>, offset: usize) -> LangResult<Vec<u8>> {
        let mut buf = ByteWriter::new(Endianness::Little);

        // Write the string structure
        buf.write_vec(vec![0_u8; 16 * data.len()]);

        for (i, value) in data.iter().enumerate() {
            let encrypted = xtea_encrypt(&value);

            let start = i * 0x10;
            buf.write((encrypted.len() | 0x40000000) as u32, start)?;
            buf.write((offset + buf.len()) as u64, start + 8)?;
            buf.write_vec(encrypted);
            self.relocations.push((offset + start + 8) as u32)
        }

        Ok(buf.buf())
    }

    fn read_string_vec(buf: &mut ByteReader) -> LangResult<Vec<String>> {
        let next = buf.cursor() + 24;
        let start: u64 = buf.read()?;
        let end: u64 = buf.read()?;
        let size = (end - start) / 16;

        buf.seek(start as usize)?;

        let mut vec: Vec<String> = Vec::new();

        for _ in 0..size {
            let len = buf.read::<u64>()? & !0x40000000;
            let ptr: u64 = buf.read()?;
            let cursor = buf.cursor();

            buf.seek(ptr as usize)?;
            vec.push(xtea_decrypt(buf.read_n(len as usize)?)?);

            buf.seek(cursor)?;
        }

        buf.seek(next)?;

        Ok(vec)
    }

    fn read_rid_vec(buf: &mut ByteReader) -> LangResult<Vec<u64>> {
        let cursor = buf.cursor() + 24;
        let start: u64 = buf.read()?;
        let end: u64 = buf.read()?;
        let size = (end - start) / 8;

        buf.seek(start as usize)?;

        let mut vec: Vec<u64> = Vec::new();

        for _ in 0..size {
            let high: u64 = buf.read::<u32>()? as u64;
            let low: u64 = buf.read::<u32>()? as u64;

            vec.push((high << 32) | low);
        }

        buf.seek(cursor)?;

        Ok(vec)
    }
}

pub struct RTLV {
    lang_map: Vec<String>,
    depends: IndexMap<String, String>,
}

impl RTLV {
    pub fn new(version: Version, lang_map: Option<String>) -> LangResult<Self> {
        let lang_map = if let Some(map) = lang_map {
            map.split(',').map(|s| s.to_string()).collect()
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

        Ok(RTLV {
            lang_map,
            depends: IndexMap::new(),
        })
    }

    pub fn convert(&self, data: &[u8], meta_json: String) -> LangResult<RtlvJson> {
        let mut buf = ByteReader::new(data, Endianness::Little);

        if buf.read::<u32>()? != 0x314E4942 {
            return Err(LangError::InvalidInput);
        }

        buf.rebase(0x10);

        let mut j = RtlvJson {
            schema: "https://tonytools.win/schemas/rtlv.schema.json".into(),
            hash: "".into(),
            langmap: None,
            videos: Map::new(),
            subtitles: Map::new(),
        };

        let data = GameRtlv::read(&mut buf)?;

        for (lang, rid) in std::iter::zip(data.video_languages, data.video_rids) {
            j.videos.insert(lang, format!("{:016X}", rid).into());
        }

        for (lang, subtitle) in std::iter::zip(data.subtitle_languages, data.subtitles) {
            j.subtitles.insert(lang, subtitle.into());
        }

        let meta: ResourceMeta = serde_json::from_str(&meta_json)?;
        j.hash = meta.hash_path.unwrap_or(meta.hash_value);

        Ok(j)
    }

    pub fn rebuild(&mut self, json: String) -> LangResult<Rebuilt> {
        self.depends.clear();

        let json: RtlvJson = serde_json::from_str(&json)?;

        if json.videos.len() < 1 {
            return Err(LangError::InvalidInput);
        }

        let mut rtlv = GameRtlv {
            video_languages: Vec::new(),
            video_rids: Vec::new(),
            subtitle_languages: Vec::new(),
            subtitles: Vec::new(),
            relocations: Vec::new(),
        };

        for (lang, video) in json.videos {
            let index = self.lang_map.iter().position(|x| *x == lang).unwrap();

            if let Some(video) = video.as_str() {
                rtlv.video_languages.push(lang);
                rtlv.video_rids.push(u64::from_str_radix(
                    &if !is_valid_hash(video) {
                        compute_hash(video)
                    } else {
                        video.to_string()
                    },
                    16,
                )?);

                self.depends
                    .insert(video.to_string(), format!("{:2X}", 0x80 + index));
            } else {
                return Err(LangError::InvalidInput);
            }
        }

        for (lang, subtitle) in json.subtitles {
            if let Some(subtitle) = subtitle.as_str() {
                rtlv.subtitle_languages.push(lang);
                rtlv.subtitles.push(subtitle.to_string());
            } else {
                return Err(LangError::InvalidInput);
            }
        }

        let buf = rtlv.serialize()?;
        Ok(Rebuilt {
            file: buf.clone(),
            meta: serde_json::to_string(&ResourceMeta::new(
                json.hash,
                buf.len() as u32,
                "RTLV".into(),
                self.depends.clone(),
            ))?,
        })
    }
}
