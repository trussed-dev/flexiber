// //! Common handling for types backed by byte slices with enforcement of the
// //! format-level length limitation of 65,535 bytes.

use crate::{Decodable, Decoder, Encodable, Encoder, ErrorKind, header::Header, Length, Result, Slice, Tag};

/// SIMPLE-TLV data object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaggedValue<V> {
    tag: Tag,
    value: V,
}

/// Raw SIMPLE-TLV data object `TaggedValue<Slice<'_>>`.
pub type TaggedSlice<'a> = TaggedValue<Slice<'a>>;

impl<V> TaggedValue<V>
{
    pub fn new(tag: Tag, value: V) -> Self {
        Self { tag, value }
    }

    pub fn tag(&self) -> Tag {
        self.tag
    }
}

impl<'a, E> TaggedValue<&'a E>
where
    E: Encodable
{
    fn header(&self) -> Result<Header> {
        Ok(Header {
            tag: self.tag(),
            length: self.value.encoded_length()?,
        })
    }
}

impl<'a, E> Encodable for TaggedValue<&'a E>
where
    E: Encodable
{
    fn encoded_length(&self) -> Result<Length> {
        self.header()?.encoded_length()? + self.value.encoded_length()?
    }
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        self.header()?.encode(encoder)?;
        encoder.encode(self.value)
    }
}

impl<'a> TaggedSlice<'a> {

    /// Create a new tagged slice, checking lengths.
    pub fn from(tag: Tag, slice: &'a [u8]) -> Result<Self> {
        Slice::new(slice)
            .map(|slice| Self { tag, value: slice })
            .map_err(|_| (ErrorKind::InvalidLength).into())
    }

    /// Borrow the inner byte slice.
    pub fn as_bytes(&self) -> &'a [u8] {
        self.value.as_bytes()
    }

    /// Get the length of the inner byte slice.
    pub fn length(&self) -> Length {
        self.value.length()
    }

    /// Is the inner byte slice empty?
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Get the SIMPLE-TLV [`Header`] for this [`TaggedSlice`] value
    fn header(&self) -> Result<Header> {
        Ok(Header {
            tag: self.tag(),
            length: self.length(),
        })
    }

    /// Decode nested values, creating a new [`Decoder`] for
    /// the data contained in the sequence's body and passing it to the provided
    /// [`FnOnce`].
    pub fn decode_nested<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Decoder<'a>) -> Result<T>,
    {
        let mut nested_decoder = Decoder::new(self.as_bytes());
        let result = f(&mut nested_decoder)?;
        nested_decoder.finish(result)
    }
}

impl<'a> Decodable<'a> for TaggedSlice<'a> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<TaggedSlice<'a>> {
        let header = Header::decode(decoder)?;
        let tag = header.tag;
        let len = header.length.to_usize();
        let value = decoder.bytes(len).map_err(|_| ErrorKind::Length { tag })?;
        Self::from(tag, value)
    }
}

impl<'a> Encodable for TaggedSlice<'a> {
    fn encoded_length(&self) -> Result<Length> {
        self.header()?.encoded_length()? + self.length()
    }

    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<()> {
        self.header()?.encode(encoder)?;
        encoder.bytes(self.as_bytes())
    }
}

// /// Obtain the length of an ASN.1 `SEQUENCE` of [`Encodable`] values when
// /// serialized as ASN.1 DER, including the `SEQUENCE` tag and length prefix.
// pub fn encoded_length2(/*tag: Tag,*/ encodables: &[&dyn Encodable]) -> Result<Length> {
//     let inner_len = Length::try_from(encodables)?;
//     Header::new(crate::tag::MEANINGLESS_TAG, inner_len)?.encoded_length() + inner_len
// }

// /// Obtain the inner length of a container of [`Encodable`] values
// /// excluding the tag and length.
// pub(crate) fn sum_encoded_lengths(encodables: &[&dyn Encodable]) -> Result<Length> {
//     encodables
//         .iter()
//         .fold(Ok(Length::zero()), |sum, encodable| {
//             sum + encodable.encoded_length()?
//         })
// }


#[cfg(test)]
mod tests {
    use core::convert::TryFrom;
    use crate::{Encodable, Tag, TaggedSlice};

    #[test]
    fn encode() {
        let mut buf = [0u8; 1024];

        let short = TaggedSlice::from(Tag::try_from(0x66).unwrap(), &[1, 2, 3]).unwrap();

        assert_eq!(
            short.encode_to_slice(&mut buf).unwrap(),
            &[0x66, 0x3, 1, 2, 3]
        );

        let slice = &[43u8; 256];
        let long = TaggedSlice::from(Tag::try_from(0x66).unwrap(), slice).unwrap();
        let encoded = long.encode_to_slice(&mut buf).unwrap();
        assert_eq!(&encoded[..4], &[0x66, 0xFF, 0x01, 0x00]);
        assert_eq!(&encoded[4..], slice);
    }
}
