use std::error::Error;
use strum_macros::Display;

use bimap::BiMap;
use bitchomp::{ByteReader, ByteReaderError, Endianness};

#[derive(Clone)]
pub struct HashList {
    pub tags: BiMap<u32, String>,
    pub switches: BiMap<u32, String>,
    pub lines: BiMap<u32, String>,
    pub version: u32,
}

#[derive(Debug, Display)]
pub enum HashListError {
    InvalidFile,
    InvalidChecksum,
    DidNotReachEOF,
    ReaderError(ByteReaderError),
}

impl From<ByteReaderError> for HashListError {
    fn from(err: ByteReaderError) -> Self {
        HashListError::ReaderError(err)
    }
}

impl Error for HashListError {}

impl HashList {
    pub fn load(data: &[u8]) -> Result<Self, HashListError> {
        let mut buf = ByteReader::new(data, Endianness::Little);
        let mut hashlist = HashList {
            lines: BiMap::new(),
            switches: BiMap::new(),
            tags: BiMap::new(),
            version: u32::MAX,
        };

        // Magic
        if buf.read::<u32>()?.inner() != 0x414C4D48 {
            return Err(HashListError::InvalidFile);
        }

        // Version
        hashlist.version = buf.read::<u32>()?.inner();

        // Checksum
        let checksum = buf.read::<u32>()?.inner();
        if checksum != crc32fast::hash(buf.cursor) {
            return Err(HashListError::InvalidChecksum);
        }

        // Soundtags
        for _ in 0..buf.read::<u32>()?.inner() {
            hashlist
                .tags
                .insert(buf.read::<u32>()?.inner(), buf.read_string()?);
        }

        // Switches
        for _ in 0..buf.read::<u32>()?.inner() {
            hashlist
                .switches
                .insert(buf.read::<u32>()?.inner(), buf.read_string()?);
        }

        // Lines
        for _ in 0..buf.read::<u32>()?.inner() {
            hashlist
                .lines
                .insert(buf.read::<u32>()?.inner(), buf.read_string()?);
        }

        Ok(hashlist)
    }

    pub fn clear(&mut self) {
        self.tags.clear();
        self.switches.clear();
        self.lines.clear();
        self.version = u32::MAX;
    }
}
