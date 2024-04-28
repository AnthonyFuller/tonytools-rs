use strum_macros::Display;
use std::error::Error;

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
        match buf.read::<u32>() {
            Ok(0x414C4D48) => {}
            Ok(_) => return Err(HashListError::InvalidFile),
            Err(e) => return Err(HashListError::ReaderError(e)),
        };

        // Version
        hashlist.version = buf.read::<u32>()?;

        // Checksum
        let checksum = buf.read::<u32>()?;
        if checksum != crc32fast::hash(buf.cursor) {
            return Err(HashListError::InvalidChecksum);
        }

        // Soundtags
        for _ in 0..buf.read::<u32>()? {
            hashlist
                .tags
                .insert(buf.read::<u32>()?, buf.read::<String>()?);
        }

        // Switches
        for _ in 0..buf.read::<u32>()? {
            hashlist
                .switches
                .insert(buf.read::<u32>()?, buf.read::<String>()?);
        }

        // Lines
        for _ in 0..buf.read::<u32>()? {
            hashlist
                .lines
                .insert(buf.read::<u32>()?, buf.read::<String>()?);
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
