use crate::util::transmutable::{Endianness, ToBytes, TryFromBytes, TryFromBytesError};

#[derive(Debug, PartialEq)]
pub struct RGB {
    r: u8,
    g: u8,
    b: u8,
}
#[derive(Debug, PartialEq)]
pub struct RGBA {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl TryFromBytes for RGB {
    type Bytes = Vec<u8>;
    type Error = TryFromBytesError;

    fn try_from_bytes(
        bytes: Self::Bytes,
        endianness: crate::util::transmutable::Endianness,
    ) -> Result<(Self, usize), Self::Error> {
        let mut data: [u8; 3] = *bytes.first_chunk().ok_or(Self::Error::ArrayFromSlice)?;
        if endianness == Endianness::Little {
            data = data.map(u8::swap_bytes);
            data.reverse();
        }
        Ok((
            Self {
                r: data[0],
                g: data[1],
                b: data[2],
            },
            3,
        ))
    }
}

impl TryFromBytes for RGBA {
    type Bytes = Vec<u8>;
    type Error = TryFromBytesError;

    fn try_from_bytes(
        bytes: Self::Bytes,
        endianness: crate::util::transmutable::Endianness,
    ) -> Result<(Self, usize), Self::Error> {
        let mut data: [u8; 4] = *bytes.first_chunk().ok_or(Self::Error::ArrayFromSlice)?;
        if endianness == Endianness::Big {
            data.reverse()
        }
        Ok((
            Self {
                r: data[0],
                g: data[1],
                b: data[2],
                a: data[3],
            },
            4,
        ))
    }
}

impl ToBytes for RGB {
    type Bytes = Vec<u8>;

    fn to_bytes(&self, endianness: Endianness) -> Self::Bytes {
        let mut data = [self.r, self.g, self.b];
        if endianness == Endianness::Big {
            data.reverse()
        }
        data.to_vec()
    }
}

impl ToBytes for RGBA {
    type Bytes = Vec<u8>;

    fn to_bytes(&self, endianness: Endianness) -> Self::Bytes {
        let mut data = [self.r, self.g, self.b, self.a];
        if endianness == Endianness::Big {
            data.reverse()
        }
        data.to_vec()
    }
}

#[cfg(test)]
use crate::util::bytereader::ByteReader;
#[cfg(test)]
use crate::util::transmutable::ByteError;

#[test]
fn rgba_test() -> Result<(), ByteError> {
    let file = std::fs::read("texture.text")?;
    let mut reader = ByteReader::new(&file, Endianness::Little);
    reader.seek(0x1EC)?;
    assert_eq!(
        reader.read::<RGBA>()?,
        RGBA {
            r: 192,
            g: 145,
            b: 128,
            a: 137
        }
    );
    Ok(())
}

#[test]
fn rgb_test() -> Result<(), ByteError> {
    let file = std::fs::read("texture.text")?;
    let mut reader = ByteReader::new(&file, Endianness::Big);
    reader.seek(0x313)?;
    assert_eq!(
        reader.read::<RGB>()?,
        RGB {
            r: 168,
            g: 160,
            b: 145,
        }
    );
    Ok(())
}
