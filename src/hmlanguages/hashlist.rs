use bimap::BiMap;
use crate::util::bytereader::{ByteReader, ByteReaderError, Endianness};

pub struct HashList {
    pub tags: BiMap<u32, String>,
    pub switches: BiMap<u32, String>,
    pub lines: BiMap<u32, String>,
    pub version: u32,
}

#[derive(Debug)]
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
        match buf.read::<u32, 4>() {
            Ok(0x414C4D48) => {}
            Ok(_) => return Err(HashListError::InvalidFile),
            Err(e) => return Err(HashListError::ReaderError(e)),
        };

        // Version
        hashlist.version = buf.read::<u32, 4>()?;

        // Checksum
        let checksum = buf.read::<u32, 4>()?;
        if checksum != crc32fast::hash(buf.cursor) {
            return Err(HashListError::InvalidChecksum);
        }

        // Soundtags
        for _ in 0..buf.read::<u32, 4>()? {
            hashlist.tags.insert(buf.read::<u32, 4>()?, buf.read_cstr()?);
        }

        // Switches
        for _ in 0..buf.read::<u32, 4>()? {
            hashlist
                .switches
                .insert(buf.read::<u32, 4>()?, buf.read_cstr()?);
        }

        // Lines
        for _ in 0..buf.read::<u32, 4>()? {
            hashlist.lines.insert(buf.read::<u32, 4>()?, buf.read_cstr()?);
        }

        return Ok(hashlist);
    }

    pub fn clear(&mut self) {
        self.tags.clear();
        self.switches.clear();
        self.lines.clear();
        self.version = u32::MAX;
    }
}

#[test]
fn test_hash_list() -> Result<(), std::io::Error> {
    let file = std::fs::read("hash_list.hmla")?;

    let hashlist = HashList::load(file.as_slice()).unwrap();

    println!(
        "{:?}",
        hashlist
            .lines
            .get_by_right("EVERGREEN_SETPIECES_GEARWALL_ITEM_GEARCAPACITYCOST_DESCRIPTION")
    );

    println!(
        "{:?}",
        hashlist
            .lines
            .get_by_left(&18554)
    );

    Ok(())
}
