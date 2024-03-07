use bimap::BiMap;
use binary_reader::BinaryReader;

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
    IOError(std::io::Error),
}

impl From<std::io::Error> for HashListError {
    fn from(err: std::io::Error) -> Self {
        HashListError::IOError(err)
    }
}

impl HashList {
    pub fn load(data: &[u8]) -> Result<Self, HashListError> {
        let mut buff = BinaryReader::from_u8(data);
        let mut hashlist = HashList {
            lines: BiMap::new(),
            switches: BiMap::new(),
            tags: BiMap::new(),
            version: u32::MAX,
        };

        // Magic
        match buff.read_u32() {
            Ok(0x484D4C41) => {}
            Ok(_) => return Err(HashListError::InvalidFile),
            Err(e) => return Err(HashListError::IOError(e)),
        };

        // Version
        hashlist.version = buff.read_u32()?;

        // Checksum
        let checksum = buff.read_u32()?;
        let pos = buff.pos;
        if checksum != crc32fast::hash(buff.read_bytes(buff.length - buff.pos)?) {
            return Err(HashListError::InvalidChecksum);
        }
        buff.jmp(pos);

        // Soundtags
        for _ in 0..buff.read_u32()? {
            hashlist.tags.insert(buff.read_u32()?, buff.read_cstr()?);
        }

        // Switches
        for _ in 0..buff.read_u32()? {
            hashlist.switches.insert(buff.read_u32()?, buff.read_cstr()?);
        }

        // Lines
        for _ in 0..buff.read_u32()? {
            hashlist.lines.insert(buff.read_u32()?, buff.read_cstr()?);
        }

        if buff.pos != buff.length {
            hashlist.clear();
            return Err(HashListError::DidNotReachEOF);
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

    println!("{:?}", hashlist.lines.get_by_right("EVERGREEN_SETPIECES_GEARWALL_ITEM_GEARCAPACITYCOST_DESCRIPTION").unwrap());

    Ok(())
}
