use core::{convert::{TryFrom, TryInto}, fmt};
use crate::{Decodable, Decoder, Encodable, Encoder, Error, ErrorKind, Length, Result, TaggedValue};

const CLASS_OFFSET: usize = 6;
const CONSTRUCTED_OFFSET: usize = 5;

/// Indicator bit for constructed form encoding (i.e. vs primitive form)
const CONSTRUCTED_FLAG: u8 = 1u8 << CONSTRUCTED_OFFSET;

/// Indicator bit for constructed form encoding (i.e. vs primitive form)
const NOT_LAST_TAG_OCTET_FLAG: u8 = 1u8 << 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
/// Class of BER tag.
pub enum Class {
    Universal = 0b00,
    Application = 0b01,
    Context = 0b10,
    Private = 0b11,
}

impl TryFrom<u8> for Class {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self> {
        use Class::*;
        Ok(match value {
            0b00 => Universal,
            0b01 => Application,
            0b10 => Context,
            0b11 => Private,
            _ => return Err(ErrorKind::InvalidClass { value }.into()),
        })
    }
}

/// The tag field consists of a single byte encoding a tag number from 1 to 254. The values '00' and 'FF' are invalid.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tag {
    pub class: Class,
    pub constructed: bool,
    pub number: u16,
}


impl Tag {
    pub const BOOLEAN: Self = Self::universal(0x1);
    pub const INTEGER: Self = Self::universal(0x1);
    pub const BIT_STRING: Self = Self::universal(0x3);
    pub const OCTET_STRING: Self = Self::universal(0x4);
    pub const NULL: Self = Self::universal(0x5);
    pub const OBJECT_IDENTIFIER: Self = Self::universal(0x6);
    pub const UTF8_STRING: Self = Self::universal(0xC);
    pub const PRINTABLE_STRING: Self = Self::universal(0x13);
    pub const UTC_TIME: Self = Self::universal(0x17);
    pub const GENERALIZED_TIME: Self = Self::universal(0x18);
    pub const SEQUENCE: Self = Self::universal(0x10).constructed();
    pub const SET: Self = Self::universal(0x11).constructed();

    pub fn from(class: Class, constructed: bool, number: u16) -> Self {
        Self { class, constructed, number }
    }
    pub const fn universal(number: u16) -> Self {
        Self { class: Class::Universal, constructed: false, number }
    }

    pub const fn application(number: u16) -> Self {
        Self { class: Class::Application, constructed: false, number }
    }

    pub const fn context(number: u16) -> Self {
        Self { class: Class::Context, constructed: false, number }
    }

    pub const fn private(number: u16) -> Self {
        Self { class: Class::Private, constructed: false, number }
    }

    pub const fn constructed(self) -> Self {
        let Self { class, constructed: _, number } = self;
        Self { class, constructed: true, number }
    }
}

impl TryFrom<&'_ [u8]> for Tag {
    type Error = Error;
    fn try_from(encoding: &[u8]) -> Result<Self> {
        let mut decoder = Decoder::new(encoding);
        decoder.decode()
    }
}

impl TryFrom<u8> for Tag {
    type Error = Error;
    fn try_from(encoded_value: u8) -> Result<Self> {
        [encoded_value].as_ref().try_into()
    }
}

/// This is the common trait that types to be used as tags
/// are supposed to implement.
pub trait TagLike: Copy + PartialEq + Sized {
    /// To stick with one Error type, make sure the tag type can somehow
    /// or other be coerced into a BerTag.
    fn embedding(self) -> Tag;

    /// Assert that this [`Tag`] matches the provided expected tag.
    ///
    /// On mismatch, returns an [`Error`] with [`ErrorKind::UnexpectedTag`].
    fn assert_eq(self, expected: Self) -> Result<Self> {
        if self == expected {
            Ok(self)
        } else {
            Err(ErrorKind::UnexpectedTag {
                expected: Some(expected.embedding()),
                actual: self.embedding(),
            }
            .into())
        }
    }

    /// Ergonomic way to get a TaggedValue for a given tag and value
    fn with_value<V>(self, value: V) -> TaggedValue<V, Self> {
        TaggedValue::new(self, value)
    }
}

impl TagLike for Tag {
    fn embedding(self) -> Tag {
        self
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // f.write_str(self.type_name())
        // write!(f, "Tag('{:02x}')", self.0)
        core::fmt::Debug::fmt(self, f)
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; 3];
        let mut encoder = Encoder::new(&mut buf);
        encoder.encode(self).unwrap();
        write!(f, "Tag(class = {:?}, constructed = {}, number = {})", self.class, self.constructed, self.number)
    }
}

impl Encodable for Tag {
    fn encoded_length(&self) -> Result<Length> {
        match self.number {
            0..=0x1E => Ok(Length(1)),
            0x1F..=0x7F => Ok(Length(2)),
            0x80..=0x3FFF => Ok(Length(3)),
            0x4000..=0xFFFF => Ok(Length(4)),
        }
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {

        let first_byte = ((self.class as u8) << CLASS_OFFSET) | ((self.constructed as u8) << CONSTRUCTED_OFFSET);

        match self.number {
            0..=0x1E => encoder.byte(first_byte | (self.number as u8)),
            0x1F..=0x7F => {
                encoder.byte(first_byte | 0x1F)?;
                encoder.byte(self.number as u8)
            }
            0x80..=0x3FFF => {
                encoder.byte(first_byte | 0x1F)?;
                encoder.byte(NOT_LAST_TAG_OCTET_FLAG | (self.number >> 7) as u8)?;
                encoder.byte((self.number & 0x7F) as u8)
            }
            0x4000..=0xFFFF => {
                todo!();
            }
        }
    }
}

impl Decodable<'_> for Tag {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
        let first_byte = decoder.byte()?;
        let class = (first_byte >> 6).try_into()?;
        let constructed = first_byte & CONSTRUCTED_FLAG != 0;
        // remove class and primitive/constructed bits
        let first_byte_masked = first_byte & ((1 << 5) - 1);

        let number = match first_byte_masked {
            number @ 0..=0x1E => {
                number as u16
            }
            _ => {
                let second_byte = decoder.byte()?;
                if second_byte & NOT_LAST_TAG_OCTET_FLAG == 0 {
                    let number = second_byte;
                    number as u16
                } else {
                    let number = second_byte & (!NOT_LAST_TAG_OCTET_FLAG);
                    let third_byte = decoder.byte()?;
                    if third_byte & NOT_LAST_TAG_OCTET_FLAG == 0 {
                        ((number as u16) << 7) | (third_byte as u16)
                    } else {
                        todo!();
                    }
                }
            }
        };
        Ok(Self { class, constructed, number })
    }
}


#[cfg(test)]
mod tests {
    use crate::{Decodable, Encodable, Tag};

    #[test]
    fn reconstruct() {
        let mut buf = [0u8; 32];

        let tag = Tag::universal(30);
        let encoded = tag.encode_to_slice(&mut buf).unwrap();
        assert_eq!(encoded, &[0x1E]);
        let tag2 = Tag::from_bytes(encoded).unwrap();
        assert_eq!(tag, tag2);

        let tag = Tag::universal(31);
        let encoded = tag.encode_to_slice(&mut buf).unwrap();
        assert_eq!(encoded, &[0x1F, 0x1F]);
        let tag2 = Tag::from_bytes(encoded).unwrap();
        assert_eq!(tag, tag2);

        let tag = Tag::universal(0xAA);
        let encoded = tag.encode_to_slice(&mut buf).unwrap();
        assert_eq!(encoded, &[0x1F, 0x81, 0x2A]);
        let tag2 = Tag::from_bytes(encoded).unwrap();
        assert_eq!(tag, tag2);

        let tag = Tag::universal(0x10).constructed();
        let encoded = tag.encode_to_slice(&mut buf).unwrap();
        // assert_eq!(encoded, &[0x1F, 0x81, 0x2A]);
        assert_eq!(encoded, &[super::CONSTRUCTED_FLAG + 0x10]);
        let tag2 = Tag::from_bytes(encoded).unwrap();
        assert_eq!(tag, tag2);
    }
}
