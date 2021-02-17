use core::{convert::TryFrom, fmt};
use crate::{Decodable, Decoder, Encodable, Encoder, Error, ErrorKind, Length, Result};

/// The tag field consists of a single byte encoding a tag number from 1 to 254. The values '00' and 'FF' are invalid.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tag(u8);

pub(crate) const MEANINGLESS_TAG: Tag = Tag(42);

impl TryFrom<u8> for Tag {
    type Error = Error;
    fn try_from(tag_number: u8) -> Result<Self> {
        match tag_number {
            byte if byte == 0 || byte == 0xFF => Err(ErrorKind::InvalidTag { byte }.into()),
            valid_tag_number => Ok(Self(valid_tag_number)),
        }
    }
}

impl Tag {
    /// Assert that this [`Tag`] matches the provided expected tag.
    ///
    /// On mismatch, returns an [`Error`] with [`ErrorKind::UnexpectedTag`].
    pub fn assert_eq(self, expected: Tag) -> Result<Tag> {
        if self == expected {
            Ok(self)
        } else {
            Err(ErrorKind::UnexpectedTag {
                expected: Some(expected),
                actual: self,
            }
            .into())
        }
    }
}

impl Decodable<'_> for Tag {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
        decoder.byte().and_then(Self::try_from)
    }
}

impl Encodable for Tag {
    fn encoded_len(&self) -> Result<Length> {
        Ok(1u8.into())
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        encoder.byte(self.0)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // f.write_str(self.type_name())
        write!(f, "Tag('{:02x}')", self.0)
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tag('{:02x}')", self.0)
    }
}
