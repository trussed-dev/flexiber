use core::convert::TryFrom;
use crate::{Decodable, Decoder, Encodable, Encoder, Error, ErrorKind, Length, Result, Tag, TagLike};

/// These are tags like in SIMPLE-TLV.
///
/// The tag field consists of a single byte encoding a tag number from 1 to 254. The values '00' and 'FF' are invalid.
///
/// The use case is that PIV (FIPS 201) data objects generally use BER-TLV, but, for historical reasons,
/// label entries with "simple" tags (in particular, tag numbers larger than 30 are still encoded
/// as single bytes.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SimpleTag(u8);

impl TryFrom<u8> for SimpleTag {
    type Error = Error;
    fn try_from(tag_number: u8) -> Result<Self> {
        match tag_number {
            byte if byte == 0 || byte == 0xFF => Err(ErrorKind::InvalidTag { byte }.into()),
            valid_tag_number => Ok(Self(valid_tag_number)),
        }
    }
}

impl TagLike for SimpleTag {
    fn embedding(self) -> Tag {
        use crate::Class::*;
        Tag { class: Universal, constructed: false, number: self.0 as u16 }
    }
}

impl Decodable<'_> for SimpleTag {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
        decoder.byte().and_then(Self::try_from)
    }
}

impl Encodable for SimpleTag {
    fn encoded_length(&self) -> Result<Length> {
        Ok(1u8.into())
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        encoder.byte(self.0)
    }
}


#[cfg(test)]
mod tests {
    use core::convert::TryFrom;
    use crate::{Encodable, SimpleTag, TaggedSlice};

    #[test]
    fn simple_tag() {
        let mut buf = [0u8; 384];

        let tag = SimpleTag::try_from(37).unwrap();
        let slice = &[1u8,2,3];
        let short = TaggedSlice::from(tag, slice).unwrap();

        assert_eq!(
            short.encode_to_slice(&mut buf).unwrap(),
            &[37, 0x3, 1, 2, 3]
        );

        let slice = &[43u8; 256];
        let long = TaggedSlice::from(tag, slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..4], &[37, 0x82, 0x01, 0x00]);
        assert_eq!(&encoded[4..], slice);
    }
}
